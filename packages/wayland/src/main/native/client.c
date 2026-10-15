#include <wayland-client.h>

struct wl_display *wayland_display_connect(const char *name) {
    return wl_display_connect(name);
}

void wayland_display_disconnect(struct wl_display *display) {
    wl_display_disconnect(display);
}
