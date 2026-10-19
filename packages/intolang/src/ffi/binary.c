#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    unsigned char *data;
    long long length;
} IntoBinary;

IntoBinary *into_binary_read(const char *path) {
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
    IntoBinary *binary = (IntoBinary *)calloc(1, sizeof(IntoBinary));
    if (!binary) {
        fclose(stream);
        return NULL;
    }
    binary->length = (long long)size;
    if (size > 0) {
        binary->data = (unsigned char *)malloc((size_t)size);
        if (!binary->data) {
            free(binary);
            fclose(stream);
            return NULL;
        }
        if (fread(binary->data, 1, (size_t)size, stream) != (size_t)size) {
            free(binary->data);
            free(binary);
            fclose(stream);
            return NULL;
        }
    }
    fclose(stream);
    return binary;
}

int32_t into_binary_write(IntoBinary *binary, const char *path) {
    if (!path || !binary) return 0;
    FILE *stream = fopen(path, "wb");
    if (!stream) return 0;
    if (binary->length > 0 &&
        fwrite(binary->data, 1, (size_t)binary->length, stream) != (size_t)binary->length) {
        fclose(stream);
        return 0;
    }
    fclose(stream);
    return 1;
}

int64_t into_binary_length(IntoBinary *binary) {
    return binary ? binary->length : 0;
}

int32_t into_binary_at(IntoBinary *binary, int64_t index) {
    if (!binary || index < 0 || index >= binary->length) return 0;
    return (int32_t)binary->data[index];
}

const char *into_binary_text(IntoBinary *binary) {
    if (!binary) return "";
    unsigned char *terminated = (unsigned char *)realloc(binary->data, (size_t)binary->length + 1);
    if (!terminated) return "";
    terminated[binary->length] = '\0';
    binary->data = terminated;
    return (const char *)binary->data;
}

void into_binary_destroy(IntoBinary *binary) {
    if (!binary) return;
    free(binary->data);
    free(binary);
}
