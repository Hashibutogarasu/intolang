#include <stdlib.h>

typedef struct {
    int placeholder;
} FsFile;

FsFile *fs_file_new(void) {
    return (FsFile *)calloc(1, sizeof(FsFile));
}

void fs_file_destroy(FsFile *file) {
    free(file);
}
