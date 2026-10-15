#define VK_USE_PLATFORM_WAYLAND_KHR
#include <vulkan/vulkan.h>
#include <wayland-client.h>

#include <algorithm>
#include <cstdint>
#include <limits>
#include <vector>

namespace {
struct DrawRect {
    VkClearRect area;
    uint32_t color;
};

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
    std::vector<VkImage> images;
    std::vector<VkImageView> views;
    VkRenderPass render_pass = VK_NULL_HANDLE;
    std::vector<VkFramebuffer> framebuffers;
    VkCommandPool command_pool = VK_NULL_HANDLE;
    std::vector<VkCommandBuffer> commands;
    VkSemaphore image_available = VK_NULL_HANDLE;
    VkSemaphore render_finished = VK_NULL_HANDLE;
    VkFence frame_fence = VK_NULL_HANDLE;
    uint32_t image_index = 0;
    int32_t clear_color = 0;
    bool frame_active = false;
    std::vector<DrawRect> rectangles;
};

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

void destroy(VulkanWindow &window) {
    if (window.device != VK_NULL_HANDLE) vkDeviceWaitIdle(window.device);
    if (window.device != VK_NULL_HANDLE && window.frame_fence != VK_NULL_HANDLE) vkDestroyFence(window.device, window.frame_fence, nullptr);
    if (window.device != VK_NULL_HANDLE && window.image_available != VK_NULL_HANDLE) vkDestroySemaphore(window.device, window.image_available, nullptr);
    if (window.device != VK_NULL_HANDLE && window.render_finished != VK_NULL_HANDLE) vkDestroySemaphore(window.device, window.render_finished, nullptr);
    if (window.device != VK_NULL_HANDLE && window.command_pool != VK_NULL_HANDLE) vkDestroyCommandPool(window.device, window.command_pool, nullptr);
    if (window.device != VK_NULL_HANDLE) {
        for (VkFramebuffer framebuffer : window.framebuffers) vkDestroyFramebuffer(window.device, framebuffer, nullptr);
        if (window.render_pass != VK_NULL_HANDLE) vkDestroyRenderPass(window.device, window.render_pass, nullptr);
        for (VkImageView view : window.views) vkDestroyImageView(window.device, view, nullptr);
        if (window.swapchain != VK_NULL_HANDLE) vkDestroySwapchainKHR(window.device, window.swapchain, nullptr);
        vkDestroyDevice(window.device, nullptr);
    }
    if (window.instance != VK_NULL_HANDLE && window.surface != VK_NULL_HANDLE) vkDestroySurfaceKHR(window.instance, window.surface, nullptr);
    if (window.instance != VK_NULL_HANDLE) vkDestroyInstance(window.instance, nullptr);
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
    swapchain_info.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    swapchain_info.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    swapchain_info.preTransform = capabilities.currentTransform;
    swapchain_info.compositeAlpha = composite;
    swapchain_info.presentMode = VK_PRESENT_MODE_FIFO_KHR;
    swapchain_info.clipped = VK_TRUE;
    if (vkCreateSwapchainKHR(window.device, &swapchain_info, nullptr, &window.swapchain) != VK_SUCCESS) return false;

    if (vkGetSwapchainImagesKHR(window.device, window.swapchain, &image_count, nullptr) != VK_SUCCESS || image_count == 0) return false;
    window.images.resize(image_count);
    if (vkGetSwapchainImagesKHR(window.device, window.swapchain, &image_count, window.images.data()) != VK_SUCCESS) return false;
    window.views.resize(image_count);
    for (uint32_t i = 0; i < image_count; ++i) {
        VkImageViewCreateInfo view_info{VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO};
        view_info.image = window.images[i];
        view_info.viewType = VK_IMAGE_VIEW_TYPE_2D;
        view_info.format = window.format;
        view_info.components = {VK_COMPONENT_SWIZZLE_IDENTITY, VK_COMPONENT_SWIZZLE_IDENTITY,
                                VK_COMPONENT_SWIZZLE_IDENTITY, VK_COMPONENT_SWIZZLE_IDENTITY};
        view_info.subresourceRange = {VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1};
        if (vkCreateImageView(window.device, &view_info, nullptr, &window.views[i]) != VK_SUCCESS) return false;
    }

    VkAttachmentDescription attachment{};
    attachment.format = window.format;
    attachment.samples = VK_SAMPLE_COUNT_1_BIT;
    attachment.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR;
    attachment.storeOp = VK_ATTACHMENT_STORE_OP_STORE;
    attachment.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
    attachment.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
    attachment.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    attachment.finalLayout = VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;
    VkAttachmentReference color_reference{0, VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL};
    VkSubpassDescription subpass{};
    subpass.pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS;
    subpass.colorAttachmentCount = 1;
    subpass.pColorAttachments = &color_reference;
    VkRenderPassCreateInfo render_pass_info{VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO};
    render_pass_info.attachmentCount = 1;
    render_pass_info.pAttachments = &attachment;
    render_pass_info.subpassCount = 1;
    render_pass_info.pSubpasses = &subpass;
    if (vkCreateRenderPass(window.device, &render_pass_info, nullptr, &window.render_pass) != VK_SUCCESS) return false;

    window.framebuffers.resize(image_count);
    for (uint32_t i = 0; i < image_count; ++i) {
        VkFramebufferCreateInfo framebuffer_info{VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO};
        framebuffer_info.renderPass = window.render_pass;
        framebuffer_info.attachmentCount = 1;
        framebuffer_info.pAttachments = &window.views[i];
        framebuffer_info.width = window.extent.width;
        framebuffer_info.height = window.extent.height;
        framebuffer_info.layers = 1;
        if (vkCreateFramebuffer(window.device, &framebuffer_info, nullptr, &window.framebuffers[i]) != VK_SUCCESS) return false;
    }

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
    return vkCreateFence(window.device, &fence_info, nullptr, &window.frame_fence) == VK_SUCCESS;
}

VkClearColorValue unpack_color(uint32_t color) {
    return VkClearColorValue{{static_cast<float>((color >> 16) & 0xff) / 255.0f,
                              static_cast<float>((color >> 8) & 0xff) / 255.0f,
                              static_cast<float>(color & 0xff) / 255.0f, 1.0f}};
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
    VkClearValue clear{};
    clear.color = unpack_color(static_cast<uint32_t>(clear_color));
    VkRenderPassBeginInfo pass_info{VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO};
    pass_info.renderPass = window->render_pass;
    pass_info.framebuffer = window->framebuffers[window->image_index];
    pass_info.renderArea = {{0, 0}, window->extent};
    pass_info.clearValueCount = 1;
    pass_info.pClearValues = &clear;
    vkCmdBeginRenderPass(command, &pass_info, VK_SUBPASS_CONTENTS_INLINE);
    window->clear_color = clear_color;
    window->rectangles.clear();
    window->frame_active = true;
}

extern "C" void vulkan_wayland_window_fill_rect(VulkanWindow *window, int32_t x, int32_t y,
                                                   int32_t width, int32_t height, int32_t color) {
    if (!window || !window->frame_active || width <= 0 || height <= 0) return;
    const int64_t left = std::max<int64_t>(0, x), top = std::max<int64_t>(0, y);
    const int64_t right = std::min<int64_t>(window->extent.width, static_cast<int64_t>(x) + width);
    const int64_t bottom = std::min<int64_t>(window->extent.height, static_cast<int64_t>(y) + height);
    if (right <= left || bottom <= top) return;
    VkClearRect area{{{static_cast<int32_t>(left), static_cast<int32_t>(top)},
                       {static_cast<uint32_t>(right - left), static_cast<uint32_t>(bottom - top)}}, 0, 1};
    window->rectangles.push_back({area, static_cast<uint32_t>(color)});
}

extern "C" void vulkan_wayland_window_present(VulkanWindow *window) {
    if (!window || !window->frame_active) return;
    VkCommandBuffer command = window->commands[window->image_index];
    for (const DrawRect &rectangle : window->rectangles) {
        VkClearAttachment attachment{};
        attachment.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        attachment.colorAttachment = 0;
        attachment.clearValue.color = unpack_color(rectangle.color);
        vkCmdClearAttachments(command, 1, &attachment, 1, &rectangle.area);
    }
    vkCmdEndRenderPass(command);
    if (vkEndCommandBuffer(command) != VK_SUCCESS) { window->frame_active = false; return; }
    if (vkResetFences(window->device, 1, &window->frame_fence) != VK_SUCCESS) { window->frame_active = false; return; }
    const VkPipelineStageFlags wait_stage = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
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
