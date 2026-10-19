#include <stdint.h>

void *dummy_process_current(void) { return 0; }

void *dummy_process_spawn(const char *path, const char *args) {
    (void)path;
    (void)args;
    return 0;
}

int32_t dummy_process_alive(int32_t pid) {
    (void)pid;
    return 0;
}

void dummy_process_advance(int32_t milliseconds) { (void)milliseconds; }
