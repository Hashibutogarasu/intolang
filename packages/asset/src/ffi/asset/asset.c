#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

extern const unsigned char into_bundle_data[] __attribute__((weak));
extern const unsigned long into_bundle_size __attribute__((weak));
extern const char into_bundle_digest[] __attribute__((weak));

#define DIGEST_FILE ".digest"
#define ENTRY_TAIL 53

static char assets_root[PATH_MAX];
static int assets_ready = 0;

static uint32_t read_u32(const unsigned char *bytes) {
    return (uint32_t)bytes[0] | (uint32_t)bytes[1] << 8 | (uint32_t)bytes[2] << 16 |
           (uint32_t)bytes[3] << 24;
}

static uint64_t read_u64(const unsigned char *bytes) {
    return (uint64_t)read_u32(bytes) | (uint64_t)read_u32(bytes + 4) << 32;
}

static void fail(const char *message, const char *detail) {
    fprintf(stderr, "%s%s\n", message, detail);
    exit(1);
}

static int make_directories(const char *path) {
    char buffer[PATH_MAX];
    size_t length = strlen(path);
    if (length >= sizeof(buffer)) {
        return -1;
    }
    memcpy(buffer, path, length + 1);
    for (size_t index = 1; index < length; index++) {
        if (buffer[index] == '/') {
            buffer[index] = '\0';
            if (mkdir(buffer, 0755) != 0 && errno != EEXIST) {
                return -1;
            }
            buffer[index] = '/';
        }
    }
    return (mkdir(buffer, 0755) != 0 && errno != EEXIST) ? -1 : 0;
}

static int write_file(const char *path, const unsigned char *contents, size_t size, uint32_t mode) {
    char parent[PATH_MAX];
    size_t length = strlen(path);
    if (length >= sizeof(parent)) {
        return -1;
    }
    memcpy(parent, path, length + 1);
    char *slash = strrchr(parent, '/');
    if (slash) {
        *slash = '\0';
        if (make_directories(parent) != 0) {
            return -1;
        }
    }
    FILE *file = fopen(path, "wb");
    if (!file) {
        return -1;
    }
    int written = size == 0 || fwrite(contents, 1, size, file) == size;
    fclose(file);
    chmod(path, mode & 0777);
    return written ? 0 : -1;
}

static int unpack(const char *root) {
    const unsigned char *bytes = into_bundle_data;
    size_t size = (size_t)into_bundle_size;
    size_t position = 0;
    if (size >= 2 && bytes[0] == '#' && bytes[1] == '!') {
        while (position < size && bytes[position] != '\n') {
            position++;
        }
        position++;
    }
    if (position + 16 > size || memcmp(bytes + position, "INEX", 4) != 0) {
        return -1;
    }
    position += 8;
    uint32_t generator = read_u32(bytes + position);
    position += 4 + (size_t)generator;
    if (position + 4 > size) {
        return -1;
    }
    uint32_t count = read_u32(bytes + position);
    position += 4;
    size_t table = position;
    for (uint32_t index = 0; index < count; index++) {
        if (position + 4 > size) {
            return -1;
        }
        position += 4 + (size_t)read_u32(bytes + position) + ENTRY_TAIL;
        if (position > size) {
            return -1;
        }
    }
    const unsigned char *data = bytes + position;
    size_t data_size = size - position;
    position = table;
    if (make_directories(root) != 0) {
        return -1;
    }
    for (uint32_t index = 0; index < count; index++) {
        uint32_t length = read_u32(bytes + position);
        char path[PATH_MAX];
        if ((size_t)length + strlen(root) + 2 >= sizeof(path)) {
            return -1;
        }
        snprintf(path, sizeof(path), "%s/%.*s", root, (int)length, (const char *)bytes + position + 4);
        position += 4 + (size_t)length;
        unsigned char kind = bytes[position];
        uint32_t mode = read_u32(bytes + position + 1);
        uint64_t file_size = read_u64(bytes + position + 5);
        uint64_t offset = read_u64(bytes + position + 13);
        position += ENTRY_TAIL;
        if (kind == 0) {
            if (make_directories(path) != 0) {
                return -1;
            }
        } else {
            if (offset > data_size || file_size > data_size - offset) {
                return -1;
            }
            if (write_file(path, data + offset, (size_t)file_size, mode) != 0) {
                return -1;
            }
        }
    }
    return 0;
}

static void prepare_assets(void) {
    if (assets_ready) {
        return;
    }
    if (!into_bundle_data) {
        fail("The program has no assets; call `require` first", "");
    }
    char executable[PATH_MAX];
    ssize_t length = readlink("/proc/self/exe", executable, sizeof(executable) - 1);
    if (length <= 0) {
        fail("Could not find the executable of the program", "");
    }
    executable[length] = '\0';
    snprintf(assets_root, sizeof(assets_root), "%s.assets", executable);
    char marker[PATH_MAX];
    snprintf(marker, sizeof(marker), "%s/" DIGEST_FILE, assets_root);
    char known[128] = {0};
    FILE *file = fopen(marker, "rb");
    if (file) {
        size_t read = fread(known, 1, sizeof(known) - 1, file);
        known[read] = '\0';
        fclose(file);
    }
    if (strcmp(known, into_bundle_digest) != 0) {
        if (unpack(assets_root) != 0) {
            fail("Could not unpack the assets to ", assets_root);
        }
        FILE *written = fopen(marker, "wb");
        if (written) {
            fputs(into_bundle_digest, written);
            fclose(written);
        }
    }
    assets_ready = 1;
}

void asset_require(const char *directory) {
    (void)directory;
}

const char *asset_path(const char *path) {
    prepare_assets();
    if (path[0] == '/' || strstr(path, "..") != NULL) {
        fail("The asset path leaves the bundle: ", path);
    }
    size_t length = strlen(assets_root) + strlen(path) + 2;
    char *full = (char *)malloc(length);
    if (!full) {
        fail("Out of memory", "");
    }
    snprintf(full, length, "%s/%s", assets_root, path);
    if (access(full, R_OK) != 0) {
        fail("The asset is not in the bundle: ", path);
    }
    return full;
}
