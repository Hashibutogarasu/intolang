#include <algorithm>
#include <cstdint>
#include <poll.h>
#include <string>

#ifdef __linux__
#include <fcntl.h>
#include <linux/memfd.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <wayland-client.h>
#include "xdg-shell-client-protocol.h"
#include "xdg-decoration-client-protocol.h"
extern "C" {
#include "xdg-shell-protocol.c"
#include "xdg-decoration-protocol.c"
}

extern "C" void vulkan_bind(void *renderer, void *pixels, int32_t width, int32_t height) __attribute__((weak));

namespace {
struct Wayland;
struct Window;

struct TitleBar {
    Window *window = nullptr;
    zxdg_toplevel_decoration_v1 *decoration = nullptr;
    bool closable = true;
};

struct Window {
    Wayland *owner = nullptr;
    wl_surface *surface = nullptr;
    xdg_surface *xdg_surface_handle = nullptr;
    xdg_toplevel *toplevel = nullptr;
    wl_buffer *buffer = nullptr;
    int fd = -1;
    int width = 0;
    int height = 0;
    int cursor_y = 0;
    uint32_t *pixels = nullptr;
    bool released = true;
    bool closed = false;
    bool created = false;
    TitleBar titlebar;
};

struct Wayland {
    wl_display *display = nullptr;
    wl_registry *registry = nullptr;
    wl_compositor *compositor = nullptr;
    xdg_wm_base *wm_base = nullptr;
    wl_shm *shm = nullptr;
    zxdg_decoration_manager_v1 *decoration_manager = nullptr;
    Window window;
    void *renderer = nullptr;
};

void registry_global(void *data, wl_registry *registry, uint32_t name, const char *interface, uint32_t version) {
    auto *wayland = static_cast<Wayland *>(data);
    if (std::string(interface) == "wl_compositor")
        wayland->compositor = static_cast<wl_compositor *>(wl_registry_bind(registry, name, &wl_compositor_interface, std::min(version, 4u)));
    else if (std::string(interface) == "xdg_wm_base")
        wayland->wm_base = static_cast<xdg_wm_base *>(wl_registry_bind(registry, name, &xdg_wm_base_interface, 1));
    else if (std::string(interface) == "wl_shm")
        wayland->shm = static_cast<wl_shm *>(wl_registry_bind(registry, name, &wl_shm_interface, 1));
    else if (std::string(interface) == "zxdg_decoration_manager_v1")
        wayland->decoration_manager = static_cast<zxdg_decoration_manager_v1 *>(wl_registry_bind(registry, name, &zxdg_decoration_manager_v1_interface, 1));
}
void registry_remove(void *, wl_registry *, uint32_t) {}
const wl_registry_listener registry_listener{registry_global, registry_remove};
void wm_ping(void *, xdg_wm_base *base, uint32_t serial) { xdg_wm_base_pong(base, serial); }
const xdg_wm_base_listener wm_listener{wm_ping};
void surface_configure(void *, xdg_surface *surface, uint32_t serial) { xdg_surface_ack_configure(surface, serial); }
const xdg_surface_listener surface_listener{surface_configure};
void toplevel_configure(void *, xdg_toplevel *, int32_t, int32_t, wl_array *) {}
void toplevel_close(void *data, xdg_toplevel *) {
    auto *window = static_cast<Window *>(data);
    if (!window->titlebar.closable) return;
    window->closed = true;
}
void toplevel_bounds(void *, xdg_toplevel *, int32_t, int32_t) {}
void toplevel_capabilities(void *, xdg_toplevel *, wl_array *) {}
const xdg_toplevel_listener toplevel_listener{toplevel_configure, toplevel_close, toplevel_bounds, toplevel_capabilities};
void buffer_release(void *data, wl_buffer *) { static_cast<Window *>(data)->released = true; }
const wl_buffer_listener buffer_listener{buffer_release};

void destroy_window(Window &window) {
    if (window.titlebar.decoration) zxdg_toplevel_decoration_v1_destroy(window.titlebar.decoration);
    if (window.buffer) wl_buffer_destroy(window.buffer);
    if (window.pixels) munmap(window.pixels, static_cast<size_t>(window.width) * window.height * sizeof(uint32_t));
    if (window.fd >= 0) close(window.fd);
    if (window.toplevel) xdg_toplevel_destroy(window.toplevel);
    if (window.xdg_surface_handle) xdg_surface_destroy(window.xdg_surface_handle);
    if (window.surface) wl_surface_destroy(window.surface);
    window = Window{};
}
}

extern "C" Wayland *wayland_new() {
    auto *wayland = new Wayland();
    wayland->window.owner = wayland;
    wayland->display = wl_display_connect(nullptr);
    if (!wayland->display) { delete wayland; return nullptr; }
    wayland->registry = wl_display_get_registry(wayland->display);
    wl_registry_add_listener(wayland->registry, &registry_listener, wayland);
    if (wl_display_roundtrip(wayland->display) < 0 || !wayland->compositor || !wayland->wm_base || !wayland->shm) {
        wl_display_disconnect(wayland->display);
        delete wayland;
        return nullptr;
    }
    xdg_wm_base_add_listener(wayland->wm_base, &wm_listener, wayland);
    return wayland;
}

extern "C" Window *wayland_window(Wayland *wayland) {
    return wayland ? &wayland->window : nullptr;
}

extern "C" void wayland_set_renderer(Wayland *wayland, void *renderer) {
    if (!wayland) return;
    wayland->renderer = renderer;
    if (vulkan_bind && wayland->window.created) {
        vulkan_bind(renderer, wayland->window.pixels, wayland->window.width, wayland->window.height);
    }
}

extern "C" void window_create(Window *window, int32_t width, int32_t height, const char *title) {
    if (!window || !window->owner || window->created || width <= 0 || height <= 0) return;
    Wayland *wayland = window->owner;
    window->width = width;
    window->height = height;
    window->surface = wl_compositor_create_surface(wayland->compositor);
    if (!window->surface) return;
    window->xdg_surface_handle = xdg_wm_base_get_xdg_surface(wayland->wm_base, window->surface);
    if (!window->xdg_surface_handle) { destroy_window(*window); return; }
    xdg_surface_add_listener(window->xdg_surface_handle, &surface_listener, window);
    window->toplevel = xdg_surface_get_toplevel(window->xdg_surface_handle);
    if (!window->toplevel) { destroy_window(*window); return; }
    xdg_toplevel_add_listener(window->toplevel, &toplevel_listener, window);
    xdg_toplevel_set_title(window->toplevel, title ? title : "Into Window");
    xdg_toplevel_set_app_id(window->toplevel, "intolang.application");
    wl_surface_commit(window->surface);
    if (wl_display_roundtrip(wayland->display) < 0) { destroy_window(*window); return; }
    const size_t bytes = static_cast<size_t>(width) * height * sizeof(uint32_t);
    window->fd = static_cast<int>(syscall(SYS_memfd_create, "into-window", MFD_CLOEXEC));
    if (window->fd < 0 || ftruncate(window->fd, static_cast<off_t>(bytes)) < 0) { destroy_window(*window); return; }
    window->pixels = static_cast<uint32_t *>(mmap(nullptr, bytes, PROT_READ | PROT_WRITE, MAP_SHARED, window->fd, 0));
    if (window->pixels == MAP_FAILED) { window->pixels = nullptr; destroy_window(*window); return; }
    wl_shm_pool *pool = wl_shm_create_pool(wayland->shm, window->fd, static_cast<int32_t>(bytes));
    window->buffer = wl_shm_pool_create_buffer(pool, 0, width, height, width * 4, WL_SHM_FORMAT_XRGB8888);
    wl_shm_pool_destroy(pool);
    if (!window->buffer) { destroy_window(*window); return; }
    wl_buffer_add_listener(window->buffer, &buffer_listener, window);
    window->created = true;
    if (vulkan_bind && wayland->renderer) {
        vulkan_bind(wayland->renderer, window->pixels, window->width, window->height);
    }
}

extern "C" void window_attach(Window *window) {
    if (!window || !window->owner || window->closed || !window->created) return;
    wl_surface_attach(window->surface, window->buffer, 0, 0);
}

extern "C" void window_damage(Window *window) {
    if (!window || !window->created) return;
    wl_surface_damage(window->surface, 0, 0, window->width, window->height);
}

extern "C" void window_commit(Window *window) {
    if (!window || !window->created) return;
    wl_surface_commit(window->surface);
}

extern "C" void window_flush(Window *window) {
    if (!window || !window->owner) return;
    wl_display_flush(window->owner->display);
}

extern "C" TitleBar *window_titlebar(Window *window) {
    if (!window) return nullptr;
    window->titlebar.window = window;
    return &window->titlebar;
}

extern "C" void window_titlebar_use_standard(TitleBar *titlebar) {
    if (!titlebar || !titlebar->window) return;
    Window *window = titlebar->window;
    Wayland *wayland = window->owner;
    if (titlebar->decoration || !wayland || !wayland->decoration_manager || !window->toplevel) return;
    titlebar->decoration = zxdg_decoration_manager_v1_get_toplevel_decoration(wayland->decoration_manager, window->toplevel);
    zxdg_toplevel_decoration_v1_set_mode(titlebar->decoration, ZXDG_TOPLEVEL_DECORATION_V1_MODE_SERVER_SIDE);
    wl_surface_commit(window->surface);
}

extern "C" void window_titlebar_allow_maximize(TitleBar *titlebar) {
    if (!titlebar || !titlebar->window || !titlebar->window->toplevel) return;
    xdg_toplevel_set_min_size(titlebar->window->toplevel, 0, 0);
    xdg_toplevel_set_max_size(titlebar->window->toplevel, 0, 0);
    wl_surface_commit(titlebar->window->surface);
}

extern "C" void window_titlebar_prevent_maximize(TitleBar *titlebar) {
    if (!titlebar || !titlebar->window || !titlebar->window->toplevel) return;
    xdg_toplevel_set_min_size(titlebar->window->toplevel, titlebar->window->width, titlebar->window->height);
    xdg_toplevel_set_max_size(titlebar->window->toplevel, titlebar->window->width, titlebar->window->height);
    wl_surface_commit(titlebar->window->surface);
}

extern "C" void window_titlebar_allow_close(TitleBar *titlebar) {
    if (titlebar) titlebar->closable = true;
}

extern "C" void window_titlebar_prevent_close(TitleBar *titlebar) {
    if (titlebar) titlebar->closable = false;
}

extern "C" void window_clear(Window *window, int32_t color) {
    if (!window || !window->pixels) return;
    std::fill(window->pixels, window->pixels + static_cast<size_t>(window->width) * window->height, 0xff000000u | (static_cast<uint32_t>(color) & 0x00ffffffu));
    window->cursor_y = 0;
}

extern "C" void window_fill_rect(Window *window, int32_t x, int32_t y, int32_t width, int32_t height, int32_t color) {
    if (!window || !window->pixels || width <= 0 || height <= 0) return;
    const int left = std::max(0, x), top = std::max(0, y);
    const int right = std::min(window->width, x + width), bottom = std::min(window->height, y + height);
    const uint32_t pixel = 0xff000000u | (static_cast<uint32_t>(color) & 0x00ffffffu);
    for (int row = top; row < bottom; ++row)
        std::fill(window->pixels + static_cast<size_t>(row) * window->width + left,
                  window->pixels + static_cast<size_t>(row) * window->width + right, pixel);
}

extern "C" void window_draw_text(Window *window, const char *text, int32_t color) {
    if (!window || !window->pixels || !text || window->cursor_y >= 60) return;
    static constexpr uint8_t asterisk_rows[] = {0b01010, 0b01010, 0b00100, 0b11111, 0b00100, 0b01010, 0b01010};
    const uint32_t pixel = 0xff000000u | (static_cast<uint32_t>(color) & 0x00ffffffu);
    const int top = window->cursor_y * 8;
    for (int y = top; y < std::min(top + 8, window->height); ++y)
        std::fill(window->pixels + static_cast<size_t>(y) * window->width,
                  window->pixels + static_cast<size_t>(y) * window->width + window->width, 0xff000000u);
    for (int column = 0; text[column] && column < 160; ++column) {
        if (text[column] != '*') continue;
        const int left = column * window->width / 160;
        const int right = (column + 1) * window->width / 160;
        const int cell_width = right - left;
        for (int glyph_y = 0; glyph_y < 7; ++glyph_y) {
            for (int glyph_x = 0; glyph_x < 5; ++glyph_x) {
                if ((asterisk_rows[glyph_y] & (1u << (4 - glyph_x))) == 0) continue;
                const int glyph_left = left + glyph_x * cell_width / 5;
                const int glyph_right = left + (glyph_x + 1) * cell_width / 5;
                const int y = top + glyph_y;
                if (y < window->height)
                    std::fill(window->pixels + static_cast<size_t>(y) * window->width + glyph_left,
                              window->pixels + static_cast<size_t>(y) * window->width + glyph_right, pixel);
            }
        }
    }
    ++window->cursor_y;
}

extern "C" void window_reset_cursor(Window *window) {
    if (window) window->cursor_y = 0;
}

extern "C" bool window_is_open(Window *window) {
    if (!window || !window->owner || window->closed) return false;
    wl_display *display = window->owner->display;
    wl_display_dispatch_pending(display);
    wl_display_flush(display);
    pollfd descriptor{wl_display_get_fd(display), POLLIN, 0};
    while (poll(&descriptor, 1, 0) > 0) {
        if (wl_display_dispatch(display) < 0) { window->closed = true; return false; }
    }
    return !window->closed;
}

extern "C" void wayland_destroy(Wayland *wayland) {
    if (!wayland) return;
    destroy_window(wayland->window);
    if (wayland->registry) wl_registry_destroy(wayland->registry);
    if (wayland->display) wl_display_disconnect(wayland->display);
    delete wayland;
}
#else
struct Wayland {};
struct Window {};
struct TitleBar {};
extern "C" Wayland *wayland_new() { return nullptr; }
extern "C" Window *wayland_window(Wayland *) { return nullptr; }
extern "C" void wayland_set_renderer(Wayland *, void *) {}
extern "C" void window_create(Window *, int32_t, int32_t, const char *) {}
extern "C" void window_clear(Window *, int32_t) {}
extern "C" void window_fill_rect(Window *, int32_t, int32_t, int32_t, int32_t, int32_t) {}
extern "C" void window_draw_text(Window *, const char *, int32_t) {}
extern "C" void window_reset_cursor(Window *) {}
extern "C" void window_attach(Window *) {}
extern "C" void window_damage(Window *) {}
extern "C" void window_commit(Window *) {}
extern "C" void window_flush(Window *) {}
extern "C" TitleBar *window_titlebar(Window *) { return nullptr; }
extern "C" void window_titlebar_use_standard(TitleBar *) {}
extern "C" void window_titlebar_allow_maximize(TitleBar *) {}
extern "C" void window_titlebar_prevent_maximize(TitleBar *) {}
extern "C" void window_titlebar_allow_close(TitleBar *) {}
extern "C" void window_titlebar_prevent_close(TitleBar *) {}
extern "C" bool window_is_open(Window *) { return false; }
extern "C" void wayland_destroy(Wayland *) {}
#endif
