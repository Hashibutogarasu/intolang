#include <stdint.h>
#include <stdlib.h>
#include <string.h>

void *into_memory_allocate(int64_t bytes) {
    if (bytes <= 0) {
        return 0;
    }
    return calloc(1, (size_t)bytes);
}

void into_memory_free(void *pointer) {
    free(pointer);
}

void into_memory_copy(void *target, const void *source, int64_t bytes) {
    if (target && source && bytes > 0) {
        memmove(target, source, (size_t)bytes);
    }
}

void into_memory_fill(void *target, int32_t value, int64_t bytes) {
    if (target && bytes > 0) {
        memset(target, value, (size_t)bytes);
    }
}
