#include <vulkan/vulkan.h>

int32_t vulkan_create_instance(const VkInstanceCreateInfo *create_info, const VkAllocationCallbacks *allocator, VkInstance *instance) {
    return (int32_t)vkCreateInstance(create_info, allocator, instance);
}

void vulkan_destroy_instance(VkInstance instance, const VkAllocationCallbacks *allocator) {
    vkDestroyInstance(instance, allocator);
}
