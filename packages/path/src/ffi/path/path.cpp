#include <filesystem>
#include <string>

namespace {

thread_local std::string buffer;

const char *store(const std::filesystem::path &value) {
    buffer = value.string();
    return buffer.c_str();
}

}  // namespace

extern "C" const char *path_join(const char *base, const char *leaf) {
    return store(std::filesystem::path(base ? base : "") / (leaf ? leaf : ""));
}

extern "C" const char *path_normalize(const char *value) {
    return store(std::filesystem::path(value ? value : "").lexically_normal());
}

extern "C" const char *path_absolute(const char *value) {
    return store(std::filesystem::absolute(value ? value : ""));
}

extern "C" const char *path_parent(const char *value) {
    return store(std::filesystem::path(value ? value : "").parent_path());
}

extern "C" const char *path_file_name(const char *value) {
    return store(std::filesystem::path(value ? value : "").filename());
}

extern "C" const char *path_extension(const char *value) {
    return store(std::filesystem::path(value ? value : "").extension());
}

extern "C" const char *path_stem(const char *value) {
    return store(std::filesystem::path(value ? value : "").stem());
}
