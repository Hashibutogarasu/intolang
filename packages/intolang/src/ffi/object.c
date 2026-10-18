#include <stdint.h>
#include <stdio.h>

#define INTO_OBJECT_TEXTS 16

int into_object_hash(void *instance) {
    uintptr_t address = (uintptr_t)instance;
    uint64_t mixed = (uint64_t)address * 0x9E3779B97F4A7C15ull;
    return (int)(mixed >> 33);
}

const char *into_object_to_string(void *instance) {
    static _Thread_local char texts[INTO_OBJECT_TEXTS][32];
    static _Thread_local unsigned next;
    char *text = texts[next++ % INTO_OBJECT_TEXTS];
    snprintf(text, sizeof texts[0], "Object@%x", (unsigned)into_object_hash(instance));
    return text;
}
