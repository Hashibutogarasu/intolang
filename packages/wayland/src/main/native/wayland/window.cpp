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
extern "C" {
#include "xdg-shell-protocol.c"
}

namespace {
struct Window {
    wl_display *display = nullptr;
    wl_compositor *compositor = nullptr;
    xdg_wm_base *wm_base = nullptr;
    wl_shm *shm = nullptr;
    wl_surface *surface = nullptr;
    xdg_surface *xdg_surface_handle = nullptr;
    xdg_toplevel *toplevel = nullptr;
    wl_buffer *buffer = nullptr;
    int fd = -1;
    int width = 0;
    int height = 0;
    uint32_t *pixels = nullptr;
    bool released = true;
    bool closed = false;
};

void registry_global(void *data, wl_registry *registry, uint32_t name, const char *interface, uint32_t version) {
    auto *window = static_cast<Window *>(data);
    if (std::string(interface) == "wl_compositor")
        window->compositor = static_cast<wl_compositor *>(wl_registry_bind(registry, name, &wl_compositor_interface, std::min(version, 4u)));
    else if (std::string(interface) == "xdg_wm_base")
        window->wm_base = static_cast<xdg_wm_base *>(wl_registry_bind(registry, name, &xdg_wm_base_interface, 1));
    else if (std::string(interface) == "wl_shm")
        window->shm = static_cast<wl_shm *>(wl_registry_bind(registry, name, &wl_shm_interface, 1));
}
void registry_remove(void *, wl_registry *, uint32_t) {}
const wl_registry_listener registry_listener{registry_global, registry_remove};
void wm_ping(void *, xdg_wm_base *base, uint32_t serial) { xdg_wm_base_pong(base, serial); }
const xdg_wm_base_listener wm_listener{wm_ping};
void surface_configure(void *, xdg_surface *surface, uint32_t serial) { xdg_surface_ack_configure(surface, serial); }
const xdg_surface_listener surface_listener{surface_configure};
void toplevel_configure(void *, xdg_toplevel *, int32_t, int32_t, wl_array *) {}
void toplevel_close(void *data, xdg_toplevel *) { static_cast<Window *>(data)->closed = true; }
void toplevel_bounds(void *, xdg_toplevel *, int32_t, int32_t) {}
void toplevel_capabilities(void *, xdg_toplevel *, wl_array *) {}
const xdg_toplevel_listener toplevel_listener{toplevel_configure, toplevel_close, toplevel_bounds, toplevel_capabilities};
void buffer_release(void *data, wl_buffer *) { static_cast<Window *>(data)->released = true; }
const wl_buffer_listener buffer_listener{buffer_release};

bool wait_for_buffer(Window &window) {
    while (!window.released && !window.closed) {
        pollfd descriptor{wl_display_get_fd(window.display), POLLIN, 0};
        if (poll(&descriptor, 1, 50) > 0 && wl_display_dispatch(window.display) < 0) return false;
        wl_display_dispatch_pending(window.display);
        wl_display_flush(window.display);
    }
    return !window.closed;
}

bool initialize(Window &window, int width, int height, const char *title) {
    if (width <= 0 || height <= 0) return false;
    window.width = width;
    window.height = height;
    window.display = wl_display_connect(nullptr);
    if (!window.display) return false;
    wl_registry *registry = wl_display_get_registry(window.display);
    wl_registry_add_listener(registry, &registry_listener, &window);
    if (wl_display_roundtrip(window.display) < 0 || !window.compositor || !window.wm_base || !window.shm) return false;
    xdg_wm_base_add_listener(window.wm_base, &wm_listener, &window);
    window.surface = wl_compositor_create_surface(window.compositor);
    if (!window.surface) return false;
    window.xdg_surface_handle = xdg_wm_base_get_xdg_surface(window.wm_base, window.surface);
    if (!window.xdg_surface_handle) return false;
    xdg_surface_add_listener(window.xdg_surface_handle, &surface_listener, &window);
    window.toplevel = xdg_surface_get_toplevel(window.xdg_surface_handle);
    if (!window.toplevel) return false;
    xdg_toplevel_add_listener(window.toplevel, &toplevel_listener, &window);
    xdg_toplevel_set_title(window.toplevel, title ? title : "Into Window");
    xdg_toplevel_set_app_id(window.toplevel, "intolang.application");
    wl_surface_commit(window.surface);
    if (wl_display_roundtrip(window.display) < 0) return false;
    const size_t bytes = static_cast<size_t>(width) * height * sizeof(uint32_t);
    window.fd = static_cast<int>(syscall(SYS_memfd_create, "into-window", MFD_CLOEXEC));
    if (window.fd < 0 || ftruncate(window.fd, static_cast<off_t>(bytes)) < 0) return false;
    window.pixels = static_cast<uint32_t *>(mmap(nullptr, bytes, PROT_READ | PROT_WRITE, MAP_SHARED, window.fd, 0));
    if (window.pixels == MAP_FAILED) { window.pixels = nullptr; return false; }
    wl_shm_pool *pool = wl_shm_create_pool(window.shm, window.fd, static_cast<int32_t>(bytes));
    window.buffer = wl_shm_pool_create_buffer(pool, 0, width, height, width * 4, WL_SHM_FORMAT_XRGB8888);
    wl_shm_pool_destroy(pool);
    if (!window.buffer) return false;
    wl_buffer_add_listener(window.buffer, &buffer_listener, &window);
    return true;
}

void destroy(Window &window) {
    if (window.display) wl_display_disconnect(window.display);
    if (window.pixels) munmap(window.pixels, static_cast<size_t>(window.width) * window.height * sizeof(uint32_t));
    if (window.fd >= 0) close(window.fd);
}
}

extern "C" Window *wayland_window_create(int32_t width, int32_t height, const char *title) {
    auto *window = new Window();
    if (!initialize(*window, width, height, title)) { destroy(*window); delete window; return nullptr; }
    return window;
}

extern "C" void wayland_window_clear(Window *window, int32_t color) {
    if (!window || !wait_for_buffer(*window)) return;
    std::fill(window->pixels, window->pixels + static_cast<size_t>(window->width) * window->height, 0xff000000u | (static_cast<uint32_t>(color) & 0x00ffffffu));
}

extern "C" void wayland_window_fill_rect(Window *window, int32_t x, int32_t y, int32_t width, int32_t height, int32_t color) {
    if (!window || !window->pixels || width <= 0 || height <= 0) return;
    const int left = std::max(0, x), top = std::max(0, y);
    const int right = std::min(window->width, x + width), bottom = std::min(window->height, y + height);
    const uint32_t pixel = 0xff000000u | (static_cast<uint32_t>(color) & 0x00ffffffu);
    for (int row = top; row < bottom; ++row)
        std::fill(window->pixels + static_cast<size_t>(row) * window->width + left,
                  window->pixels + static_cast<size_t>(row) * window->width + right, pixel);
}

extern "C" void wayland_window_present(Window *window) {
    if (!window || window->closed || !wait_for_buffer(*window)) return;
    window->released = false;
    wl_surface_attach(window->surface, window->buffer, 0, 0);
    wl_surface_damage(window->surface, 0, 0, window->width, window->height);
    wl_surface_commit(window->surface);
    wl_display_flush(window->display);
}

extern "C" bool wayland_window_is_open(Window *window) {
    if (!window || window->closed) return false;
    pollfd descriptor{wl_display_get_fd(window->display), POLLIN, 0};
    if (poll(&descriptor, 1, 0) > 0) {
        if (wl_display_dispatch(window->display) < 0) { window->closed = true; return false; }
    }
    wl_display_dispatch_pending(window->display);
    return !window->closed;
}

extern "C" void wayland_window_destroy(Window *window) {
    if (!window) return;
    destroy(*window);
    delete window;
}
#else
struct Window {};
extern "C" Window *wayland_window_create(int32_t, int32_t, const char *) { return nullptr; }
extern "C" void wayland_window_clear(Window *, int32_t) {}
extern "C" void wayland_window_fill_rect(Window *, int32_t, int32_t, int32_t, int32_t, int32_t) {}
extern "C" void wayland_window_present(Window *) {}
extern "C" bool wayland_window_is_open(Window *) { return false; }
extern "C" void wayland_window_destroy(Window *) {}
#endif
