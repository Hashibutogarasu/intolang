#include <algorithm>
#include <cstdint>

namespace {
constexpr uint8_t GLYPH_SIZE = 8;

const uint8_t *glyph_for(char character) {
    static const uint8_t blank[8] = {0, 0, 0, 0, 0, 0, 0, 0};
    static const uint8_t dot[8] = {0, 0, 0, 0, 0, 0x18, 0x18, 0};
    static const uint8_t colon[8] = {0, 0x18, 0x18, 0, 0x18, 0x18, 0, 0};
    static const uint8_t dash[8] = {0, 0, 0, 0x7e, 0, 0, 0, 0};
    static const uint8_t equals[8] = {0, 0, 0x7e, 0, 0x7e, 0, 0, 0};
    static const uint8_t plus[8] = {0, 0, 0x18, 0x7e, 0x18, 0, 0, 0};
    static const uint8_t star[8] = {0, 0x24, 0x18, 0x7e, 0x18, 0x24, 0, 0};
    static const uint8_t hash[8] = {0, 0x24, 0x7e, 0x24, 0x7e, 0x24, 0, 0};
    static const uint8_t percent[8] = {0, 0x62, 0x64, 0x08, 0x10, 0x26, 0x46, 0};
    static const uint8_t at[8] = {0x3c, 0x42, 0x5a, 0x52, 0x5e, 0x40, 0x3c, 0};
    switch (character) {
        case '.': return dot;
        case ':': return colon;
        case '-': return dash;
        case '=': return equals;
        case '+': return plus;
        case '*': return star;
        case '#': return hash;
        case '%': return percent;
        case '@': return at;
        default: return blank;
    }
}

struct VulkanWindow {
    uint32_t *pixels = nullptr;
    int32_t width = 0;
    int32_t height = 0;
    int cursor_x = 0;
    int cursor_y = 0;
};

uint32_t pack_color(int32_t color) {
    return 0xff000000u | (static_cast<uint32_t>(color) & 0x00ffffffu);
}
}

extern "C" VulkanWindow *vulkan_new() {
    return new VulkanWindow();
}

extern "C" void vulkan_bind(VulkanWindow *window, void *pixels, int32_t width, int32_t height) {
    if (!window) return;
    window->pixels = static_cast<uint32_t *>(pixels);
    window->width = width;
    window->height = height;
}

extern "C" void vulkan_begin_frame(VulkanWindow *window, int32_t clear_color) {
    if (!window || !window->pixels || window->width <= 0 || window->height <= 0) return;
    std::fill(window->pixels, window->pixels + static_cast<size_t>(window->width) * window->height, pack_color(clear_color));
    window->cursor_x = 0;
    window->cursor_y = 0;
}

extern "C" void vulkan_fill_rect(VulkanWindow *window, int32_t x, int32_t y, int32_t width, int32_t height, int32_t color) {
    if (!window || !window->pixels || width <= 0 || height <= 0) return;
    const int left = std::max(0, x);
    const int top = std::max(0, y);
    const int right = std::min(window->width, x + width);
    const int bottom = std::min(window->height, y + height);
    if (right <= left || bottom <= top) return;
    const uint32_t pixel = pack_color(color);
    for (int row = top; row < bottom; ++row) {
        std::fill(window->pixels + static_cast<size_t>(row) * window->width + left,
                  window->pixels + static_cast<size_t>(row) * window->width + right, pixel);
    }
}

extern "C" void vulkan_draw_text(VulkanWindow *window, const char *text, int32_t color) {
    if (!window || !window->pixels || !text) return;
    const uint32_t pixel = pack_color(color);
    int columns = 0;
    while (text[columns] != '\0' && text[columns] != '\n') ++columns;
    int rows = 0;
    for (const char *character = text; *character != '\0'; ++character) {
        if (*character == '\n') ++rows;
    }
    if (columns > 0 || rows > 0) ++rows;
    const int text_width = columns * GLYPH_SIZE;
    const int text_height = rows * GLYPH_SIZE;
    const int start_x = (window->width - text_width) / 2;
    int cursor_x = start_x;
    int cursor_y = (window->height - text_height) / 2;
    for (const char *character = text; *character != '\0'; ++character) {
        if (*character == '\n') {
            cursor_x = start_x;
            cursor_y += GLYPH_SIZE;
            continue;
        }
        const uint8_t *glyph = glyph_for(*character);
        for (int glyph_y = 0; glyph_y < GLYPH_SIZE; ++glyph_y) {
            const uint8_t bits = glyph[glyph_y];
            if (bits == 0) continue;
            const int pixel_y = cursor_y + glyph_y;
            if (pixel_y < 0 || pixel_y >= window->height) continue;
            for (int glyph_x = 0; glyph_x < GLYPH_SIZE; ++glyph_x) {
                if ((bits & (1u << (GLYPH_SIZE - 1 - glyph_x))) == 0) continue;
                const int pixel_x = cursor_x + glyph_x;
                if (pixel_x < 0 || pixel_x >= window->width) continue;
                window->pixels[static_cast<size_t>(pixel_y) * window->width + pixel_x] = pixel;
            }
        }
        cursor_x += GLYPH_SIZE;
    }
    window->cursor_x = cursor_x;
    window->cursor_y = cursor_y;
}

extern "C" void vulkan_destroy(VulkanWindow *window) {
    if (!window) return;
    delete window;
}
