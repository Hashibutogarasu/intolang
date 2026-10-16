#define VK_USE_PLATFORM_WAYLAND_KHR
#include <vulkan/vulkan.h>
#include <wayland-client.h>

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <limits>
#include <vector>

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
    VkInstance instance = VK_NULL_HANDLE;
    VkSurfaceKHR surface = VK_NULL_HANDLE;
    VkPhysicalDevice physical_device = VK_NULL_HANDLE;
    VkDevice device = VK_NULL_HANDLE;
    VkQueue queue = VK_NULL_HANDLE;
    uint32_t queue_family = 0;
    VkSwapchainKHR swapchain = VK_NULL_HANDLE;
    VkFormat format = VK_FORMAT_UNDEFINED;
    VkExtent2D extent{};
    bool swap_red_blue = false;
    std::vector<VkImage> images;
    VkCommandPool command_pool = VK_NULL_HANDLE;
    std::vector<VkCommandBuffer> commands;
    VkSemaphore image_available = VK_NULL_HANDLE;
    VkSemaphore render_finished = VK_NULL_HANDLE;
    VkFence frame_fence = VK_NULL_HANDLE;
    VkBuffer staging_buffer = VK_NULL_HANDLE;
    VkDeviceMemory staging_memory = VK_NULL_HANDLE;
    void *staging_mapped = nullptr;
    VkDeviceSize staging_size = 0;
    std::vector<uint32_t> pixels;
    uint32_t image_index = 0;
    int32_t clear_color = 0;
    int cursor_x = 0;
    int cursor_y = 0;
    bool frame_active = false;
};

uint32_t pack_color(uint32_t color, bool swap_red_blue) {
    const uint32_t red = (color >> 16) & 0xff;
    const uint32_t green = (color >> 8) & 0xff;
    const uint32_t blue = color & 0xff;
    if (swap_red_blue) return 0xff000000u | (red << 16) | (green << 8) | blue;
    return 0xff000000u | (blue << 16) | (green << 8) | red;
}

bool choose_device(VulkanWindow &window) {
    uint32_t count = 0;
    if (vkEnumeratePhysicalDevices(window.instance, &count, nullptr) != VK_SUCCESS || count == 0) return false;
    std::vector<VkPhysicalDevice> devices(count);
    if (vkEnumeratePhysicalDevices(window.instance, &count, devices.data()) != VK_SUCCESS) return false;
    for (VkPhysicalDevice device : devices) {
        uint32_t family_count = 0;
        vkGetPhysicalDeviceQueueFamilyProperties(device, &family_count, nullptr);
        std::vector<VkQueueFamilyProperties> families(family_count);
        vkGetPhysicalDeviceQueueFamilyProperties(device, &family_count, families.data());
        for (uint32_t index = 0; index < family_count; ++index) {
            VkBool32 present = VK_FALSE;
            if ((families[index].queueFlags & VK_QUEUE_GRAPHICS_BIT) != 0 &&
                vkGetPhysicalDeviceSurfaceSupportKHR(device, index, window.surface, &present) == VK_SUCCESS && present) {
                window.physical_device = device;
                window.queue_family = index;
                return true;
            }
        }
    }
    return false;
}

uint32_t find_memory_type(VkPhysicalDevice device, uint32_t filter, VkMemoryPropertyFlags properties) {
    VkPhysicalDeviceMemoryProperties memory{};
    vkGetPhysicalDeviceMemoryProperties(device, &memory);
    for (uint32_t index = 0; index < memory.memoryTypeCount; ++index) {
        if ((filter & (1u << index)) != 0 && (memory.memoryTypes[index].propertyFlags & properties) == properties) {
            return index;
        }
    }
    return 0;
}

void destroy(VulkanWindow &window) {
    if (window.device != VK_NULL_HANDLE) vkDeviceWaitIdle(window.device);
    if (window.device != VK_NULL_HANDLE && window.staging_buffer != VK_NULL_HANDLE) vkDestroyBuffer(window.device, window.staging_buffer, nullptr);
    if (window.device != VK_NULL_HANDLE && window.staging_memory != VK_NULL_HANDLE) vkFreeMemory(window.device, window.staging_memory, nullptr);
    if (window.device != VK_NULL_HANDLE && window.frame_fence != VK_NULL_HANDLE) vkDestroyFence(window.device, window.frame_fence, nullptr);
    if (window.device != VK_NULL_HANDLE && window.image_available != VK_NULL_HANDLE) vkDestroySemaphore(window.device, window.image_available, nullptr);
    if (window.device != VK_NULL_HANDLE && window.render_finished != VK_NULL_HANDLE) vkDestroySemaphore(window.device, window.render_finished, nullptr);
    if (window.device != VK_NULL_HANDLE && window.command_pool != VK_NULL_HANDLE) vkDestroyCommandPool(window.device, window.command_pool, nullptr);
    if (window.device != VK_NULL_HANDLE) {
        if (window.swapchain != VK_NULL_HANDLE) vkDestroySwapchainKHR(window.device, window.swapchain, nullptr);
        vkDestroyDevice(window.device, nullptr);
    }
    if (window.instance != VK_NULL_HANDLE && window.surface != VK_NULL_HANDLE) vkDestroySurfaceKHR(window.instance, window.surface, nullptr);
    if (window.instance != VK_NULL_HANDLE) vkDestroyInstance(window.instance, nullptr);
}

bool create_staging(VulkanWindow &window) {
    window.staging_size = static_cast<VkDeviceSize>(window.extent.width) * window.extent.height * sizeof(uint32_t);
    window.pixels.assign(static_cast<size_t>(window.extent.width) * window.extent.height, 0xff000000u);
    VkBufferCreateInfo buffer_info{VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO};
    buffer_info.size = window.staging_size;
    buffer_info.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
    buffer_info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    if (vkCreateBuffer(window.device, &buffer_info, nullptr, &window.staging_buffer) != VK_SUCCESS) return false;
    VkMemoryRequirements requirements{};
    vkGetBufferMemoryRequirements(window.device, window.staging_buffer, &requirements);
    VkMemoryAllocateInfo allocation{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO};
    allocation.allocationSize = requirements.size;
    allocation.memoryTypeIndex = find_memory_type(window.physical_device, requirements.memoryTypeBits,
                                                  VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT);
    if (vkAllocateMemory(window.device, &allocation, nullptr, &window.staging_memory) != VK_SUCCESS) return false;
    if (vkBindBufferMemory(window.device, window.staging_buffer, window.staging_memory, 0) != VK_SUCCESS) return false;
    return vkMapMemory(window.device, window.staging_memory, 0, window.staging_size, 0, &window.staging_mapped) == VK_SUCCESS;
}

bool initialize(VulkanWindow &window, wl_display *display, wl_surface *wayland_surface, uint32_t width, uint32_t height) {
    if (!display || !wayland_surface || width == 0 || height == 0) return false;

    const char *instance_extensions[] = {VK_KHR_SURFACE_EXTENSION_NAME, VK_KHR_WAYLAND_SURFACE_EXTENSION_NAME};
    VkApplicationInfo application{VK_STRUCTURE_TYPE_APPLICATION_INFO};
    application.pApplicationName = "Into Wayland Window";
    application.applicationVersion = VK_MAKE_VERSION(1, 0, 0);
    application.pEngineName = "Into";
    application.engineVersion = VK_MAKE_VERSION(1, 0, 0);
    application.apiVersion = VK_API_VERSION_1_0;
    VkInstanceCreateInfo instance_info{VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO};
    instance_info.pApplicationInfo = &application;
    instance_info.enabledExtensionCount = 2;
    instance_info.ppEnabledExtensionNames = instance_extensions;
    if (vkCreateInstance(&instance_info, nullptr, &window.instance) != VK_SUCCESS) return false;

    VkWaylandSurfaceCreateInfoKHR surface_info{VK_STRUCTURE_TYPE_WAYLAND_SURFACE_CREATE_INFO_KHR};
    surface_info.display = display;
    surface_info.surface = wayland_surface;
    if (vkCreateWaylandSurfaceKHR(window.instance, &surface_info, nullptr, &window.surface) != VK_SUCCESS) return false;
    if (!choose_device(window)) return false;

    const float priority = 1.0f;
    VkDeviceQueueCreateInfo queue_info{VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO};
    queue_info.queueFamilyIndex = window.queue_family;
    queue_info.queueCount = 1;
    queue_info.pQueuePriorities = &priority;
    const char *device_extensions[] = {VK_KHR_SWAPCHAIN_EXTENSION_NAME};
    VkDeviceCreateInfo device_info{VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO};
    device_info.queueCreateInfoCount = 1;
    device_info.pQueueCreateInfos = &queue_info;
    device_info.enabledExtensionCount = 1;
    device_info.ppEnabledExtensionNames = device_extensions;
    if (vkCreateDevice(window.physical_device, &device_info, nullptr, &window.device) != VK_SUCCESS) return false;
    vkGetDeviceQueue(window.device, window.queue_family, 0, &window.queue);

    VkSurfaceCapabilitiesKHR capabilities{};
    if (vkGetPhysicalDeviceSurfaceCapabilitiesKHR(window.physical_device, window.surface, &capabilities) != VK_SUCCESS) return false;
    uint32_t format_count = 0;
    if (vkGetPhysicalDeviceSurfaceFormatsKHR(window.physical_device, window.surface, &format_count, nullptr) != VK_SUCCESS || format_count == 0) return false;
    std::vector<VkSurfaceFormatKHR> formats(format_count);
    if (vkGetPhysicalDeviceSurfaceFormatsKHR(window.physical_device, window.surface, &format_count, formats.data()) != VK_SUCCESS) return false;
    VkSurfaceFormatKHR chosen_format = formats.front();
    for (const auto &format : formats) {
        if (format.format == VK_FORMAT_B8G8R8A8_UNORM && format.colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR) {
            chosen_format = format;
            break;
        }
    }
    window.format = chosen_format.format;
    window.swap_red_blue = window.format == VK_FORMAT_B8G8R8A8_UNORM || window.format == VK_FORMAT_B8G8R8A8_SRGB;
    window.extent = capabilities.currentExtent.width != std::numeric_limits<uint32_t>::max()
        ? capabilities.currentExtent
        : VkExtent2D{std::clamp(width, capabilities.minImageExtent.width, capabilities.maxImageExtent.width),
                     std::clamp(height, capabilities.minImageExtent.height, capabilities.maxImageExtent.height)};
    uint32_t image_count = capabilities.minImageCount + 1;
    if (capabilities.maxImageCount > 0) image_count = std::min(image_count, capabilities.maxImageCount);
    VkCompositeAlphaFlagBitsKHR composite = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    if ((capabilities.supportedCompositeAlpha & composite) == 0) {
        for (VkCompositeAlphaFlagBitsKHR candidate : {VK_COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR,
                 VK_COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR, VK_COMPOSITE_ALPHA_INHERIT_BIT_KHR}) {
            if ((capabilities.supportedCompositeAlpha & candidate) != 0) { composite = candidate; break; }
        }
    }
    VkSwapchainCreateInfoKHR swapchain_info{VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR};
    swapchain_info.surface = window.surface;
    swapchain_info.minImageCount = image_count;
    swapchain_info.imageFormat = chosen_format.format;
    swapchain_info.imageColorSpace = chosen_format.colorSpace;
    swapchain_info.imageExtent = window.extent;
    swapchain_info.imageArrayLayers = 1;
    swapchain_info.imageUsage = VK_IMAGE_USAGE_TRANSFER_DST_BIT;
    swapchain_info.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    swapchain_info.preTransform = capabilities.currentTransform;
    swapchain_info.compositeAlpha = composite;
    swapchain_info.presentMode = VK_PRESENT_MODE_FIFO_KHR;
    swapchain_info.clipped = VK_TRUE;
    if (vkCreateSwapchainKHR(window.device, &swapchain_info, nullptr, &window.swapchain) != VK_SUCCESS) return false;

    if (vkGetSwapchainImagesKHR(window.device, window.swapchain, &image_count, nullptr) != VK_SUCCESS || image_count == 0) return false;
    window.images.resize(image_count);
    if (vkGetSwapchainImagesKHR(window.device, window.swapchain, &image_count, window.images.data()) != VK_SUCCESS) return false;

    VkCommandPoolCreateInfo pool_info{VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO};
    pool_info.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;
    pool_info.queueFamilyIndex = window.queue_family;
    if (vkCreateCommandPool(window.device, &pool_info, nullptr, &window.command_pool) != VK_SUCCESS) return false;
    window.commands.resize(image_count);
    VkCommandBufferAllocateInfo command_info{VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO};
    command_info.commandPool = window.command_pool;
    command_info.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    command_info.commandBufferCount = image_count;
    if (vkAllocateCommandBuffers(window.device, &command_info, window.commands.data()) != VK_SUCCESS) return false;

    VkSemaphoreCreateInfo semaphore_info{VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO};
    if (vkCreateSemaphore(window.device, &semaphore_info, nullptr, &window.image_available) != VK_SUCCESS ||
        vkCreateSemaphore(window.device, &semaphore_info, nullptr, &window.render_finished) != VK_SUCCESS) return false;
    VkFenceCreateInfo fence_info{VK_STRUCTURE_TYPE_FENCE_CREATE_INFO};
    fence_info.flags = VK_FENCE_CREATE_SIGNALED_BIT;
    if (vkCreateFence(window.device, &fence_info, nullptr, &window.frame_fence) != VK_SUCCESS) return false;

    return create_staging(window);
}
}

extern "C" VulkanWindow *vulkan_wayland_window_create(void *display, void *surface, int32_t width, int32_t height) {
    auto *window = new VulkanWindow();
    if (!initialize(*window, static_cast<wl_display *>(display), static_cast<wl_surface *>(surface),
                    static_cast<uint32_t>(width), static_cast<uint32_t>(height))) {
        destroy(*window);
        delete window;
        return nullptr;
    }
    return window;
}

extern "C" void vulkan_wayland_window_begin_frame(VulkanWindow *window, int32_t clear_color) {
    if (!window || window->frame_active || vkWaitForFences(window->device, 1, &window->frame_fence, VK_TRUE, UINT64_MAX) != VK_SUCCESS) return;
    const VkResult acquired = vkAcquireNextImageKHR(window->device, window->swapchain, UINT64_MAX,
                                                    window->image_available, VK_NULL_HANDLE, &window->image_index);
    if (acquired != VK_SUCCESS && acquired != VK_SUBOPTIMAL_KHR) return;
    VkCommandBuffer command = window->commands[window->image_index];
    if (vkResetCommandBuffer(command, 0) != VK_SUCCESS) return;
    VkCommandBufferBeginInfo begin_info{VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO};
    begin_info.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;
    if (vkBeginCommandBuffer(command, &begin_info) != VK_SUCCESS) return;
    VkImageMemoryBarrier barrier{VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER};
    barrier.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    barrier.newLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
    barrier.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    barrier.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    barrier.image = window->images[window->image_index];
    barrier.subresourceRange = {VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1};
    barrier.srcAccessMask = 0;
    barrier.dstAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
    vkCmdPipelineBarrier(command, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0,
                         0, nullptr, 0, nullptr, 1, &barrier);
    std::fill(window->pixels.begin(), window->pixels.end(), pack_color(static_cast<uint32_t>(clear_color), window->swap_red_blue));
    window->clear_color = clear_color;
    window->cursor_x = 0;
    window->cursor_y = 0;
    window->frame_active = true;
}

extern "C" void vulkan_wayland_window_fill_rect(VulkanWindow *window, int32_t x, int32_t y,
                                                int32_t width, int32_t height, int32_t color) {
    if (!window || !window->frame_active || width <= 0 || height <= 0) return;
    const int left = std::max<int64_t>(0, x);
    const int top = std::max<int64_t>(0, y);
    const int right = std::min<int64_t>(window->extent.width, static_cast<int64_t>(x) + width);
    const int bottom = std::min<int64_t>(window->extent.height, static_cast<int64_t>(y) + height);
    if (right <= left || bottom <= top) return;
    const uint32_t pixel = pack_color(static_cast<uint32_t>(color), window->swap_red_blue);
    for (int row = top; row < bottom; ++row) {
        std::fill(window->pixels.begin() + static_cast<size_t>(row) * window->extent.width + left,
                  window->pixels.begin() + static_cast<size_t>(row) * window->extent.width + right, pixel);
    }
}

extern "C" void vulkan_wayland_window_draw_text(VulkanWindow *window, const char *text, int32_t color) {
    if (!window || !window->frame_active || !text) return;
    const uint32_t pixel = pack_color(static_cast<uint32_t>(color), window->swap_red_blue);
    int columns = 0;
    while (text[columns] != '\0' && text[columns] != '\n') ++columns;
    int rows = 0;
    const size_t length = std::strlen(text);
    for (size_t index = 0; index < length; ++index) {
        if (text[index] == '\n') ++rows;
    }
    if (length > 0 && text[length - 1] != '\n') ++rows;
    const int text_width = columns * GLYPH_SIZE;
    const int text_height = rows * GLYPH_SIZE;
    int cursor_x = (static_cast<int>(window->extent.width) - text_width) / 2;
    int cursor_y = (static_cast<int>(window->extent.height) - text_height) / 2;
    const int start_x = cursor_x;
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
            if (pixel_y < 0 || pixel_y >= static_cast<int>(window->extent.height)) continue;
            for (int glyph_x = 0; glyph_x < GLYPH_SIZE; ++glyph_x) {
                if ((bits & (1u << (GLYPH_SIZE - 1 - glyph_x))) == 0) continue;
                const int pixel_x = cursor_x + glyph_x;
                if (pixel_x < 0 || pixel_x >= static_cast<int>(window->extent.width)) continue;
                window->pixels[static_cast<size_t>(pixel_y) * window->extent.width + pixel_x] = pixel;
            }
        }
        cursor_x += GLYPH_SIZE;
    }
    window->cursor_x = cursor_x;
    window->cursor_y = cursor_y;
}

extern "C" void vulkan_wayland_window_present(VulkanWindow *window) {
    if (!window || !window->frame_active) return;
    VkCommandBuffer command = window->commands[window->image_index];
    std::memcpy(window->staging_mapped, window->pixels.data(), static_cast<size_t>(window->staging_size));
    VkBufferImageCopy region{};
    region.imageSubresource = {VK_IMAGE_ASPECT_COLOR_BIT, 0, 0, 1};
    region.imageOffset = {0, 0, 0};
    region.imageExtent = {window->extent.width, window->extent.height, 1};
    vkCmdCopyBufferToImage(command, window->staging_buffer, window->images[window->image_index],
                           VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 1, &region);
    VkImageMemoryBarrier barrier{VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER};
    barrier.oldLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
    barrier.newLayout = VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;
    barrier.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    barrier.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    barrier.image = window->images[window->image_index];
    barrier.subresourceRange = {VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1};
    barrier.srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
    barrier.dstAccessMask = 0;
    vkCmdPipelineBarrier(command, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, 0,
                         0, nullptr, 0, nullptr, 1, &barrier);
    if (vkEndCommandBuffer(command) != VK_SUCCESS) { window->frame_active = false; return; }
    if (vkResetFences(window->device, 1, &window->frame_fence) != VK_SUCCESS) { window->frame_active = false; return; }
    const VkPipelineStageFlags wait_stage = VK_PIPELINE_STAGE_TRANSFER_BIT;
    VkSubmitInfo submit{VK_STRUCTURE_TYPE_SUBMIT_INFO};
    submit.waitSemaphoreCount = 1;
    submit.pWaitSemaphores = &window->image_available;
    submit.pWaitDstStageMask = &wait_stage;
    submit.commandBufferCount = 1;
    submit.pCommandBuffers = &command;
    submit.signalSemaphoreCount = 1;
    submit.pSignalSemaphores = &window->render_finished;
    if (vkQueueSubmit(window->queue, 1, &submit, window->frame_fence) == VK_SUCCESS) {
        VkPresentInfoKHR present{VK_STRUCTURE_TYPE_PRESENT_INFO_KHR};
        present.waitSemaphoreCount = 1;
        present.pWaitSemaphores = &window->render_finished;
        present.swapchainCount = 1;
        present.pSwapchains = &window->swapchain;
        present.pImageIndices = &window->image_index;
        vkQueuePresentKHR(window->queue, &present);
    }
    window->frame_active = false;
}

extern "C" void vulkan_wayland_window_destroy(VulkanWindow *window) {
    if (!window) return;
    destroy(*window);
    delete window;
}
