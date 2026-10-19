#include <stdlib.h>
#include <string.h>

static char *into_string_copy(const char *text, size_t length) {
    char *copy = (char *)malloc(length + 1);
    if (!copy) {
        return "";
    }
    memcpy(copy, text, length);
    copy[length] = '\0';
    return copy;
}

const char *into_string_concat(const char *left, const char *right) {
    size_t left_length = strlen(left);
    size_t right_length = strlen(right);
    char *joined = (char *)malloc(left_length + right_length + 1);
    if (!joined) {
        return "";
    }
    memcpy(joined, left, left_length);
    memcpy(joined + left_length, right, right_length);
    joined[left_length + right_length] = '\0';
    return joined;
}

const char *into_string_from_char(int code) {
    char encoded[5] = {0};
    unsigned value = (unsigned)code;
    size_t length;
    if (value < 0x80) {
        encoded[0] = (char)value;
        length = 1;
    } else if (value < 0x800) {
        encoded[0] = (char)(0xC0 | (value >> 6));
        encoded[1] = (char)(0x80 | (value & 0x3F));
        length = 2;
    } else if (value < 0x10000) {
        encoded[0] = (char)(0xE0 | (value >> 12));
        encoded[1] = (char)(0x80 | ((value >> 6) & 0x3F));
        encoded[2] = (char)(0x80 | (value & 0x3F));
        length = 3;
    } else {
        encoded[0] = (char)(0xF0 | (value >> 18));
        encoded[1] = (char)(0x80 | ((value >> 12) & 0x3F));
        encoded[2] = (char)(0x80 | ((value >> 6) & 0x3F));
        encoded[3] = (char)(0x80 | (value & 0x3F));
        length = 4;
    }
    return into_string_copy(encoded, length);
}

int into_string_equals(const char *left, const char *right) {
    return strcmp(left, right) == 0;
}

int into_string_length(const char *text) {
    return (int)strlen(text);
}

int into_string_at(const char *text, int index) {
    if (index < 0 || (size_t)index >= strlen(text)) {
        return 0;
    }
    return (unsigned char)text[index];
}

const char *into_string_substring(const char *text, int start, int end) {
    int length = (int)strlen(text);
    if (start < 0) {
        start = 0;
    }
    if (end > length) {
        end = length;
    }
    if (start >= end) {
        return into_string_copy("", 0);
    }
    return into_string_copy(text + start, (size_t)(end - start));
}

int into_string_index_of(const char *text, const char *needle) {
    const char *found = strstr(text, needle);
    return found ? (int)(found - text) : -1;
}

int into_string_starts_with(const char *text, const char *prefix) {
    return strncmp(text, prefix, strlen(prefix)) == 0;
}

int into_string_ends_with(const char *text, const char *suffix) {
    size_t length = strlen(text);
    size_t suffix_length = strlen(suffix);
    return suffix_length <= length && strcmp(text + length - suffix_length, suffix) == 0;
}

const char *into_string_to_lower(const char *text) {
    size_t length = strlen(text);
    char *lowered = into_string_copy(text, length);
    for (size_t index = 0; index < length; index++) {
        if (lowered[index] >= 'A' && lowered[index] <= 'Z') {
            lowered[index] = (char)(lowered[index] + 32);
        }
    }
    return lowered;
}

const char *into_string_to_upper(const char *text) {
    size_t length = strlen(text);
    char *raised = into_string_copy(text, length);
    for (size_t index = 0; index < length; index++) {
        if (raised[index] >= 'a' && raised[index] <= 'z') {
            raised[index] = (char)(raised[index] - 32);
        }
    }
    return raised;
}
