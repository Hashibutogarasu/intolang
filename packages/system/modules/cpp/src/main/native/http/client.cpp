#include <cerrno>
#include <algorithm>
#include <cctype>
#include <cstring>
#include <cstdlib>
#include <iostream>
#include <netdb.h>
#include <string>
#include <sys/socket.h>
#include <unistd.h>

struct HttpResponse { const char *html; };

static HttpResponse *http_error(const std::string &message) {
    return new HttpResponse{::strdup(message.c_str())};
}

extern "C" HttpResponse *http(int method, const char *url) {
    if (method != 0) {
        std::cerr << "Unsupported HTTP method\n";
        return http_error("Unsupported HTTP method");
    }
    if (!url || std::strncmp(url, "http://", 7) != 0) {
        std::cerr << "http accepts an http:// URL\n";
        return http_error("http accepts an http:// URL");
    }
    const char *host_start = url + 7;
    const char *path_start = std::strchr(host_start, '/');
    std::string host = path_start ? std::string(host_start, path_start) : host_start;
    std::string path = path_start ? path_start : "/";
    addrinfo hints{};
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    addrinfo *addresses = nullptr;
    int status = getaddrinfo(host.c_str(), "80", &hints, &addresses);
    if (status != 0) return http_error(std::string("DNS lookup failed: ") + gai_strerror(status));
    int fd = -1;
    for (addrinfo *address = addresses; address; address = address->ai_next) {
        fd = socket(address->ai_family, address->ai_socktype, address->ai_protocol);
        if (fd >= 0 && connect(fd, address->ai_addr, address->ai_addrlen) == 0) break;
        if (fd >= 0) close(fd);
        fd = -1;
    }
    freeaddrinfo(addresses);
    if (fd < 0) return http_error(std::string("Could not connect to ") + host + ": " + std::strerror(errno));
    std::string request = "GET " + path + " HTTP/1.1\r\nHost: " + host +
        "\r\nUser-Agent: Into/0.1\r\nConnection: close\r\n\r\n";
    size_t offset = 0;
    while (offset < request.size()) {
        ssize_t count = send(fd, request.data() + offset, request.size() - offset, 0);
        if (count <= 0) break;
        offset += static_cast<size_t>(count);
    }
    char buffer[4096];
    ssize_t count;
    std::string response;
    while ((count = recv(fd, buffer, sizeof(buffer), 0)) > 0) response.append(buffer, static_cast<size_t>(count));
    close(fd);
    const size_t body = response.find("\r\n\r\n");
    if (body == std::string::npos) return http_error("Malformed HTTP response");
    std::string headers = response.substr(0, body);
    std::transform(headers.begin(), headers.end(), headers.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    std::string html = response.substr(body + 4);
    if (headers.find("transfer-encoding: chunked") != std::string::npos) {
        std::string decoded;
        size_t offset = 0;
        while (offset < html.size()) {
            const size_t line_end = html.find("\r\n", offset);
            if (line_end == std::string::npos) break;
            const std::string size_text = html.substr(offset, line_end - offset);
            const size_t chunk_size = std::stoul(size_text, nullptr, 16);
            offset = line_end + 2;
            if (chunk_size == 0 || offset + chunk_size > html.size()) break;
            decoded.append(html, offset, chunk_size);
            offset += chunk_size + 2;
        }
        html = std::move(decoded);
    }
    auto *result = new HttpResponse{::strdup(html.c_str())};
    return result;
}
