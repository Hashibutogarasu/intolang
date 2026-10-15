#include <cstdint>
#include <cstdlib>
#include <cstring>
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
struct JsonDocument { JsonValue *root; };

namespace {
std::vector<std::unique_ptr<JsonValue>> values;
std::vector<std::unique_ptr<JsonDocument>> documents;
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

extern "C" JsonDocument *json_parse(const char *source) {
    auto document = std::make_unique<JsonDocument>();
    document->root = Parser(source).parse();
    auto *result = document.get();
    documents.push_back(std::move(document));
    return result;
}
extern "C" JsonValue *json_root(JsonDocument *document) { return document ? document->root : nullptr; }
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
