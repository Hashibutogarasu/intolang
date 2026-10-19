#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern const unsigned char into_bundle_data[] __attribute__((weak));
extern const unsigned long into_bundle_size __attribute__((weak));

#define ENTRY_TAIL 53

typedef struct {
    unsigned char *data;
    long long length;
} IntoBinary;

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

/* Find the file `path` in the embedded bundle and give where its contents are
   and how long they are; 0 when the bundle does not hold it. */
static int find(const char *path, const unsigned char **contents, size_t *length) {
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
        fail("The bundle of the program is broken", "");
    }
    position += 8;
    position += 4 + (size_t)read_u32(bytes + position);
    if (position + 4 > size) {
        fail("The bundle of the program is broken", "");
    }
    uint32_t count = read_u32(bytes + position);
    position += 4;
    size_t table = position;
    for (uint32_t index = 0; index < count; index++) {
        if (position + 4 > size) {
            fail("The bundle of the program is broken", "");
        }
        position += 4 + (size_t)read_u32(bytes + position) + ENTRY_TAIL;
        if (position > size) {
            fail("The bundle of the program is broken", "");
        }
    }
    const unsigned char *data = bytes + position;
    size_t data_size = size - position;
    size_t wanted = strlen(path);
    position = table;
    for (uint32_t index = 0; index < count; index++) {
        size_t name = read_u32(bytes + position);
        const unsigned char *entry = bytes + position + 4 + name;
        position += 4 + name + ENTRY_TAIL;
        if (name != wanted || memcmp(bytes + position - ENTRY_TAIL - name, path, name) != 0) {
            continue;
        }
        uint64_t file_size = read_u64(entry + 5);
        uint64_t offset = read_u64(entry + 13);
        if (entry[0] != 1 || offset > data_size || file_size > data_size - offset) {
            return 0;
        }
        *contents = data + offset;
        *length = (size_t)file_size;
        return 1;
    }
    return 0;
}

void asset_require(const char *directory) {
    (void)directory;
}

const char *asset_path(const char *path) {
    return path;
}

IntoBinary *asset_read(const char *path) {
    if (!into_bundle_data) {
        fail("The program has no assets; call `require` first", "");
    }
    if (path[0] == '/' || strstr(path, "..") != NULL) {
        fail("The asset path leaves the bundle: ", path);
    }
    const unsigned char *contents = NULL;
    size_t length = 0;
    if (!find(path, &contents, &length)) {
        fail("The asset is not in the bundle: ", path);
    }
    IntoBinary *binary = (IntoBinary *)calloc(1, sizeof(IntoBinary));
    if (!binary) {
        fail("Out of memory", "");
    }
    binary->length = (long long)length;
    if (length > 0) {
        binary->data = (unsigned char *)malloc(length);
        if (!binary->data) {
            fail("Out of memory", "");
        }
        memcpy(binary->data, contents, length);
    }
    return binary;
}
