#include <algorithm>
#include <chrono>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <poll.h>
#include <string>
#include <thread>
#include <vector>

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
#endif

#ifdef __linux__
namespace {
struct Window {
    wl_display *display = nullptr;
    wl_compositor *compositor = nullptr;
    xdg_wm_base *wm_base = nullptr;
    wl_shm *shm = nullptr;
    wl_surface *surface = nullptr;
    xdg_surface *xdg_surface = nullptr;
    xdg_toplevel *toplevel = nullptr;
    wl_buffer *buffer = nullptr;
    int fd = -1;
    uint32_t *pixels = nullptr;
    bool released = true;
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
void toplevel_close(void *data, xdg_toplevel *) { static_cast<Window *>(data)->released = true; }
void toplevel_bounds(void *, xdg_toplevel *, int32_t, int32_t) {}
void toplevel_capabilities(void *, xdg_toplevel *, wl_array *) {}
const xdg_toplevel_listener toplevel_listener{toplevel_configure, toplevel_close, toplevel_bounds, toplevel_capabilities};
void buffer_release(void *data, wl_buffer *) { static_cast<Window *>(data)->released = true; }
const wl_buffer_listener buffer_listener{buffer_release};

bool create_window(Window &window) {
    window.display = wl_display_connect(nullptr);
    if (!window.display) return false;
    wl_registry *registry = wl_display_get_registry(window.display);
    wl_registry_add_listener(registry, &registry_listener, &window);
    wl_display_roundtrip(window.display);
    if (!window.compositor || !window.wm_base || !window.shm) return false;
    xdg_wm_base_add_listener(window.wm_base, &wm_listener, &window);
    window.surface = wl_compositor_create_surface(window.compositor);
    if (!window.surface) return false;
    window.xdg_surface = xdg_wm_base_get_xdg_surface(window.wm_base, window.surface);
    if (!window.xdg_surface) return false;
    xdg_surface_add_listener(window.xdg_surface, &surface_listener, &window);
    window.toplevel = xdg_surface_get_toplevel(window.xdg_surface);
    if (!window.toplevel) return false;
    xdg_toplevel_add_listener(window.toplevel, &toplevel_listener, &window);
    xdg_toplevel_set_title(window.toplevel, "Into Wayland Frame Viewer");
    xdg_toplevel_set_app_id(window.toplevel, "intolang.bad-apple");
    wl_surface_commit(window.surface);
    if (wl_display_roundtrip(window.display) < 0) return false;
    constexpr int width = 854, height = 480;
    size_t bytes = width * height * sizeof(uint32_t);
    window.fd = static_cast<int>(syscall(SYS_memfd_create, "into-frame", MFD_CLOEXEC));
    if (window.fd < 0 || ftruncate(window.fd, bytes) < 0) return false;
    window.pixels = static_cast<uint32_t *>(mmap(nullptr, bytes, PROT_READ | PROT_WRITE, MAP_SHARED, window.fd, 0));
    if (window.pixels == MAP_FAILED) { window.pixels = nullptr; return false; }
    wl_shm_pool *pool = wl_shm_create_pool(window.shm, window.fd, bytes);
    window.buffer = wl_shm_pool_create_buffer(pool, 0, width, height, width * 4, WL_SHM_FORMAT_XRGB8888);
    wl_shm_pool_destroy(pool);
    wl_buffer_add_listener(window.buffer, &buffer_listener, &window);
    return true;
}

std::vector<std::string> parse_json_rows(const std::string &json) {
    std::vector<std::string> rows;
    bool quoted = false;
    std::string row;
    for (size_t i = 0; i < json.size(); ++i) {
        char ch = json[i];
        if (!quoted && ch == '"') { quoted = true; row.clear(); continue; }
        if (quoted && ch == '"') { quoted = false; rows.push_back(row); continue; }
        if (quoted && ch == '\\' && i + 1 < json.size()) { row.push_back(json[++i]); continue; }
        if (quoted) row.push_back(ch);
    }
    return rows;
}

void show_frame(Window &window, const std::vector<std::string> &rows) {
    constexpr int width = 854, height = 480, cell = 10;
    std::fill(window.pixels, window.pixels + width * height, 0xff000000u);
    for (int y = 0; y < std::min<int>(45, rows.size()); ++y)
        for (int x = 0; x < std::min<int>(80, rows[y].size()); ++x) {
            if (rows[y][x] == ' ' || rows[y][x] == '.' || rows[y][x] == '0') continue;
            for (int dy = 0; dy < cell; ++dy)
                for (int dx = 0; dx < cell; ++dx)
                    window.pixels[(15 + y * cell + dy) * width + 27 + x * cell + dx] = 0xffffffffu;
        }
    window.released = false;
    wl_surface_attach(window.surface, window.buffer, 0, 0);
    wl_surface_damage(window.surface, 0, 0, width, height);
    wl_surface_commit(window.surface);
    wl_display_flush(window.display);
    while (!window.released) {
        pollfd fd{wl_display_get_fd(window.display), POLLIN, 0};
        poll(&fd, 1, 20);
        wl_display_dispatch_pending(window.display);
        wl_display_flush(window.display);
    }
}

void destroy_window(Window &window) {
    if (window.display) wl_display_disconnect(window.display);
    if (window.pixels) munmap(window.pixels, 854 * 480 * sizeof(uint32_t));
    if (window.fd >= 0) close(window.fd);
}
}
#endif

extern "C" void presentFrames(const char *directory) {
#ifndef __linux__
    (void)directory;
    std::cerr << "Wayland frame presentation is only available on Linux\n";
#else
    if (!directory) { std::cerr << "Frame directory is missing\n"; return; }
    std::vector<std::filesystem::path> files;
    for (const auto &entry : std::filesystem::directory_iterator(directory))
        if (entry.path().extension() == ".json") files.push_back(entry.path());
    std::sort(files.begin(), files.end());
    if (files.empty()) { std::cerr << "No JSON frame files found\n"; return; }
    Window window;
    if (!create_window(window)) { destroy_window(window); std::cerr << "Could not create a Wayland window\n"; return; }
    for (const auto &file : files) {
        auto started = std::chrono::steady_clock::now();
        std::ifstream input(file);
        std::string json((std::istreambuf_iterator<char>(input)), {});
        show_frame(window, parse_json_rows(json));
        std::this_thread::sleep_until(started + std::chrono::milliseconds(67));
    }
    destroy_window(window);
#endif
}
