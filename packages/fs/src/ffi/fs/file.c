#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    unsigned char *data;
    long long length;
} FsBytes;

typedef struct {
    int placeholder;
} FsFile;

FsFile *fs_file_new(void) {
    return (FsFile *)calloc(1, sizeof(FsFile));
}

FsBytes *fs_file_read(FsFile *file, const char *path) {
    (void)file;
    if (!path) return NULL;
    FILE *stream = fopen(path, "rb");
    if (!stream) return NULL;
    if (fseek(stream, 0, SEEK_END) != 0) {
        fclose(stream);
        return NULL;
    }
    long size = ftell(stream);
    if (size < 0) {
        fclose(stream);
        return NULL;
    }
    rewind(stream);
    FsBytes *bytes = (FsBytes *)calloc(1, sizeof(FsBytes));
    if (!bytes) {
        fclose(stream);
        return NULL;
    }
    bytes->length = (long long)size;
    if (size > 0) {
        bytes->data = (unsigned char *)malloc((size_t)size);
        if (!bytes->data) {
            free(bytes);
            fclose(stream);
            return NULL;
        }
        if (fread(bytes->data, 1, (size_t)size, stream) != (size_t)size) {
            free(bytes->data);
            free(bytes);
            fclose(stream);
            return NULL;
        }
    }
    fclose(stream);
    return bytes;
}

int32_t fs_file_write(FsFile *file, const char *path, FsBytes *bytes, int32_t format) {
    (void)file;
    (void)format;
    if (!path || !bytes) return 0;
    FILE *stream = fopen(path, "wb");
    if (!stream) return 0;
    if (bytes->length > 0 &&
        fwrite(bytes->data, 1, (size_t)bytes->length, stream) != (size_t)bytes->length) {
        fclose(stream);
        return 0;
    }
    fclose(stream);
    return 1;
}

void fs_file_destroy(FsFile *file) {
    free(file);
}

int64_t fs_bytes_length(FsBytes *bytes) {
    return bytes ? bytes->length : 0;
}

int32_t fs_bytes_at(FsBytes *bytes, int64_t index) {
    if (!bytes || index < 0 || index >= bytes->length) return 0;
    return (int32_t)bytes->data[index];
}

const char *fs_bytes_text(FsBytes *bytes) {
    if (!bytes) return "";
    unsigned char *terminated = (unsigned char *)realloc(bytes->data, (size_t)bytes->length + 1);
    if (!terminated) return "";
    terminated[bytes->length] = '\0';
    bytes->data = terminated;
    return (const char *)bytes->data;
}

void fs_bytes_destroy(FsBytes *bytes) {
    if (!bytes) return;
    free(bytes->data);
    free(bytes);
}
