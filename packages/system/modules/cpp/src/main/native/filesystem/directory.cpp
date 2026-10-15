#include <algorithm>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <memory>
#include <thread>
#include <chrono>
#include <string>
#include <vector>

struct FileIterator {
    std::vector<std::filesystem::path> entries;
    size_t index = 0;
};

extern "C" FileIterator *filesystem_list(const char *path) {
    auto result = std::make_unique<FileIterator>();
    if (path && std::filesystem::exists(path)) {
        for (const auto &entry : std::filesystem::directory_iterator(path)) {
            if (entry.is_regular_file()) result->entries.push_back(entry.path());
        }
    }
    std::sort(result->entries.begin(), result->entries.end());
    return result.release();
}

extern "C" bool filesystem_has_next(FileIterator *iterator) {
    return iterator && iterator->index < iterator->entries.size();
}

extern "C" const char *filesystem_next(FileIterator *iterator) {
    if (!filesystem_has_next(iterator)) return "";
    thread_local std::string path;
    path = iterator->entries[iterator->index++].string();
    return path.c_str();
}

extern "C" const char *filesystem_read_text(const char *path) {
    thread_local std::string contents;
    std::ifstream input(path ? path : "", std::ios::binary);
    contents.assign(std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>());
    return contents.c_str();
}

extern "C" void standard_sleep(int32_t milliseconds) {
    if (milliseconds > 0) std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
}
