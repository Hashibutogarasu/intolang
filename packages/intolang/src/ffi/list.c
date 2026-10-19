#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef union {
    int64_t integer;
    double real;
    void *pointer;
} into_item;

typedef struct {
    into_item *items;
    long length;
    long capacity;
} into_list;

void *into_list_new(void) {
    return calloc(1, sizeof(into_list));
}

static void into_list_push(into_list *list, into_item item) {
    if (list->length == list->capacity) {
        long capacity = list->capacity == 0 ? 8 : list->capacity * 2;
        into_item *items = (into_item *)realloc(list->items, (size_t)capacity * sizeof(into_item));
        if (!items) {
            return;
        }
        list->items = items;
        list->capacity = capacity;
    }
    list->items[list->length++] = item;
}

static into_item into_list_item(into_list *list, int index) {
    into_item empty;
    memset(&empty, 0, sizeof empty);
    if (index < 0 || index >= list->length) {
        return empty;
    }
    return list->items[index];
}

static void into_list_store(into_list *list, int index, into_item item) {
    if (index < 0 || index >= list->length) {
        return;
    }
    list->items[index] = item;
}

void into_list_push_int(void *handle, int64_t value) {
    into_item item;
    memset(&item, 0, sizeof item);
    item.integer = value;
    into_list_push((into_list *)handle, item);
}

int64_t into_list_get_int(void *handle, int index) {
    return into_list_item((into_list *)handle, index).integer;
}

void into_list_set_int(void *handle, int index, int64_t value) {
    into_item item;
    memset(&item, 0, sizeof item);
    item.integer = value;
    into_list_store((into_list *)handle, index, item);
}

void into_list_push_float(void *handle, double value) {
    into_item item;
    memset(&item, 0, sizeof item);
    item.real = value;
    into_list_push((into_list *)handle, item);
}

double into_list_get_float(void *handle, int index) {
    return into_list_item((into_list *)handle, index).real;
}

void into_list_set_float(void *handle, int index, double value) {
    into_item item;
    memset(&item, 0, sizeof item);
    item.real = value;
    into_list_store((into_list *)handle, index, item);
}

void into_list_push_pointer(void *handle, void *value) {
    into_item item;
    memset(&item, 0, sizeof item);
    item.pointer = value;
    into_list_push((into_list *)handle, item);
}

void *into_list_get_pointer(void *handle, int index) {
    return into_list_item((into_list *)handle, index).pointer;
}

void into_list_set_pointer(void *handle, int index, void *value) {
    into_item item;
    memset(&item, 0, sizeof item);
    item.pointer = value;
    into_list_store((into_list *)handle, index, item);
}

int into_list_length(void *handle) {
    return (int)((into_list *)handle)->length;
}

void into_list_free(void *handle) {
    into_list *list = (into_list *)handle;
    if (!list) {
        return;
    }
    free(list->items);
    free(list);
}
