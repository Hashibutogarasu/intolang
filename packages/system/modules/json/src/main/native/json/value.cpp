#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

struct JsonValue {
    int32_t kind;
    const char *text;
    std::vector<JsonValue *> items;
    std::unordered_map<std::string, JsonValue *> properties;
};
struct JsonIterator { JsonValue *value; size_t index; };
struct TextLineIterator { std::string text; size_t cursor; size_t index; };
struct TextByteIterator { std::string text; size_t index; };

namespace {
std::vector<std::unique_ptr<JsonValue>> values;
std::vector<std::unique_ptr<JsonIterator>> iterators;
std::vector<std::unique_ptr<TextLineIterator>> text_iterators;
std::vector<std::unique_ptr<TextByteIterator>> byte_iterators;
std::vector<std::unique_ptr<std::string>> strings;

JsonValue *make_value(int32_t kind, std::string text = {}) {
    auto value = std::make_unique<JsonValue>();
    value->kind = kind;
    auto content = std::make_unique<std::string>(std::move(text));
    value->text = content->c_str();
    strings.push_back(std::move(content));
    auto *result = value.get();
    values.push_back(std::move(value));
    return result;
}

class Parser {
    const char *source;
    size_t cursor = 0;
    void whitespace() { while (source[cursor] == ' ' || source[cursor] == '\n' || source[cursor] == '\r' || source[cursor] == '\t') ++cursor; }
    bool take(char ch) { whitespace(); if (source[cursor] != ch) return false; ++cursor; return true; }
    std::string string() {
        std::string out;
        if (!take('"')) return out;
        while (source[cursor] && source[cursor] != '"') {
            char ch = source[cursor++];
            if (ch == '\\') {
                char escaped = source[cursor++];
                switch (escaped) {
                    case 'n': out.push_back('\n'); break;
                    case 'r': out.push_back('\r'); break;
                    case 't': out.push_back('\t'); break;
                    case 'b': out.push_back('\b'); break;
                    case 'f': out.push_back('\f'); break;
                    default: out.push_back(escaped); break;
                }
            } else out.push_back(ch);
        }
        if (source[cursor] == '"') ++cursor;
        return out;
    }
    JsonValue *value() {
        whitespace();
        if (source[cursor] == '"') return make_value(3, string());
        if (source[cursor] == '{') {
            ++cursor;
            auto *object = make_value(5);
            whitespace();
            while (source[cursor] && source[cursor] != '}') {
                auto key = string();
                if (!take(':')) break;
                object->properties.emplace(std::move(key), value());
                whitespace();
                if (!take(',')) break;
            }
            take('}');
            return object;
        }
        if (source[cursor] == '[') {
            ++cursor;
            auto *array = make_value(4);
            whitespace();
            while (source[cursor] && source[cursor] != ']') {
                array->items.push_back(value());
                whitespace();
                if (!take(',')) break;
            }
            take(']');
            return array;
        }
        const size_t start = cursor;
        while (source[cursor] && source[cursor] != ',' && source[cursor] != ']' && source[cursor] != '}' && source[cursor] != ' ' && source[cursor] != '\n') ++cursor;
        std::string token(source + start, cursor - start);
        const int32_t kind = token == "null" ? 0 : (token == "true" || token == "false" ? 1 : 2);
        return make_value(kind, std::move(token));
    }
public:
    explicit Parser(const char *input) : source(input ? input : "") {}
    JsonValue *parse() { return value(); }
};
}

extern "C" JsonValue *json_parse(const char *source) { return Parser(source).parse(); }
extern "C" JsonValue *json_read_file(const char *path, int32_t format) {
    std::ifstream input(path ? path : "", std::ios::binary);
    std::string contents{std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>()};
    if (format == 0) return make_value(3, std::move(contents));
    return Parser(contents.c_str()).parse();
}
extern "C" const char *json_read_text_file(const char *path) {
    thread_local std::string contents;
    std::ifstream input(path ? path : "", std::ios::binary);
    contents.assign(std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>());
    return contents.c_str();
}
extern "C" JsonValue *json_get(JsonValue *value, const char *key) {
    if (!value || !key) return nullptr;
    auto found = value->properties.find(key);
    return found == value->properties.end() ? nullptr : found->second;
}
extern "C" JsonValue *json_at(JsonValue *value, int32_t index) {
    return value && index >= 0 && static_cast<size_t>(index) < value->items.size() ? value->items[static_cast<size_t>(index)] : nullptr;
}
extern "C" int32_t json_length(JsonValue *value) {
    if (!value) return 0;
    if (value->kind == 4) return static_cast<int32_t>(value->items.size());
    if (value->kind == 5) return static_cast<int32_t>(value->properties.size());
    return 0;
}
extern "C" JsonIterator *json_iterator(JsonValue *value) {
    auto iterator = std::make_unique<JsonIterator>(JsonIterator{value, 0});
    auto *result = iterator.get();
    iterators.push_back(std::move(iterator));
    return result;
}
extern "C" bool json_iterator_has_next(JsonIterator *iterator) {
    if (!iterator || !iterator->value) return false;
    if (iterator->value->kind == 4) return iterator->index < iterator->value->items.size();
    if (iterator->value->kind == 3) return iterator->index < std::strlen(iterator->value->text);
    return false;
}
extern "C" JsonValue *json_iterator_next(JsonIterator *iterator) {
    if (!json_iterator_has_next(iterator)) return nullptr;
    if (iterator->value->kind == 4) return iterator->value->items[iterator->index++];
    const unsigned char byte = static_cast<unsigned char>(iterator->value->text[iterator->index++]);
    return make_value(2, std::to_string(byte));
}
extern "C" int32_t json_iterator_index(JsonIterator *iterator) {
    return iterator && iterator->index > 0 ? static_cast<int32_t>(iterator->index - 1) : 0;
}
extern "C" int32_t json_number(JsonValue *value) {
    if (!value || value->kind != 2) return 0;
    char *end = nullptr;
    long parsed = std::strtol(value->text, &end, 10);
    return end == value->text ? 0 : static_cast<int32_t>(parsed);
}
extern "C" TextLineIterator *text_lines(const char *text) {
    auto iterator = std::make_unique<TextLineIterator>(TextLineIterator{text ? text : "", 0, 0});
    auto *result = iterator.get();
    text_iterators.push_back(std::move(iterator));
    return result;
}
extern "C" bool text_line_iterator_has_next(TextLineIterator *iterator) {
    return iterator && iterator->cursor < iterator->text.size();
}
extern "C" const char *text_line_iterator_next(TextLineIterator *iterator) {
    thread_local std::string line;
    if (!text_line_iterator_has_next(iterator)) return "";
    const size_t end = iterator->text.find('\n', iterator->cursor);
    const size_t length = (end == std::string::npos ? iterator->text.size() : end) - iterator->cursor;
    line.assign(iterator->text, iterator->cursor, length);
    if (!line.empty() && line.back() == '\r') line.pop_back();
    iterator->cursor = end == std::string::npos ? iterator->text.size() : end + 1;
    ++iterator->index;
    return line.c_str();
}
extern "C" int32_t text_line_iterator_index(TextLineIterator *iterator) {
    if (!iterator || iterator->index == 0) return -1;
    return static_cast<int32_t>(iterator->index - 1);
}
extern "C" void text_line_iterator_destroy(TextLineIterator *iterator) {
    for (auto entry = text_iterators.begin(); entry != text_iterators.end(); ++entry) {
        if (entry->get() == iterator) { text_iterators.erase(entry); return; }
    }
}
extern "C" TextByteIterator *text_bytes(const char *text) {
    auto iterator = std::make_unique<TextByteIterator>(TextByteIterator{text ? text : "", 0});
    auto *result = iterator.get();
    byte_iterators.push_back(std::move(iterator));
    return result;
}
extern "C" bool text_byte_iterator_has_next(TextByteIterator *iterator) {
    return iterator && iterator->index < iterator->text.size();
}
extern "C" int32_t text_byte_iterator_next(TextByteIterator *iterator) {
    if (!text_byte_iterator_has_next(iterator)) return -1;
    return static_cast<unsigned char>(iterator->text[iterator->index++]);
}
extern "C" int32_t text_byte_iterator_index(TextByteIterator *iterator) {
    if (!iterator || iterator->index == 0) return -1;
    return static_cast<int32_t>(iterator->index - 1);
}
extern "C" void text_byte_iterator_destroy(TextByteIterator *iterator) {
    for (auto entry = byte_iterators.begin(); entry != byte_iterators.end(); ++entry) {
        if (entry->get() == iterator) { byte_iterators.erase(entry); return; }
    }
}
extern "C" bool text_equals(const char *left, const char *right) {
    return left && right && std::strcmp(left, right) == 0;
}
