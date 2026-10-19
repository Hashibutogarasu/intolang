#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void into_console_write(const char *text) {
    fputs(text, stdout);
    fflush(stdout);
}

const char *into_console_read(void) {
    char *line = NULL;
    size_t capacity = 0;
    ssize_t length = getline(&line, &capacity, stdin);
    if (length < 0) {
        free(line);
        return "";
    }
    while (length > 0 && (line[length - 1] == '\n' || line[length - 1] == '\r')) {
        line[--length] = '\0';
    }
    return line;
}
