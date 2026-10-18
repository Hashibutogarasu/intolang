#include <stdlib.h>

typedef struct {
    void **items;
    long length;
    long capacity;
} into_list;

void *into_list_new(void) {
    return calloc(1, sizeof(into_list));
}

void into_list_push(void *handle, void *value) {
    into_list *list = (into_list *)handle;
    if (list->length == list->capacity) {
        long capacity = list->capacity == 0 ? 8 : list->capacity * 2;
        void **items = (void **)realloc(list->items, (size_t)capacity * sizeof(void *));
        if (!items) {
            return;
        }
        list->items = items;
        list->capacity = capacity;
    }
    list->items[list->length++] = value;
}

void *into_list_get(void *handle, int index) {
    into_list *list = (into_list *)handle;
    if (index < 0 || index >= list->length) {
        return 0;
    }
    return list->items[index];
}

void into_list_set(void *handle, int index, void *value) {
    into_list *list = (into_list *)handle;
    if (index < 0 || index >= list->length) {
        return;
    }
    list->items[index] = value;
}

int into_list_length(void *handle) {
    into_list *list = (into_list *)handle;
    return (int)list->length;
}

void into_list_free(void *handle) {
    into_list *list = (into_list *)handle;
    if (!list) {
        return;
    }
    free(list->items);
    free(list);
}
