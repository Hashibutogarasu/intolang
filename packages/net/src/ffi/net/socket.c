#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

typedef struct {
    int fd;
} NetSocket;

static NetSocket *net_wrap(int fd) {
    if (fd < 0) {
        return NULL;
    }
    NetSocket *socket_handle = (NetSocket *)calloc(1, sizeof(NetSocket));
    if (!socket_handle) {
        close(fd);
        return NULL;
    }
    socket_handle->fd = fd;
    return socket_handle;
}

NetSocket *net_listen(int port) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return NULL;
    }
    int reuse = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));
    struct sockaddr_in address;
    memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    address.sin_port = htons((unsigned short)port);
    if (bind(fd, (struct sockaddr *)&address, sizeof(address)) != 0 || listen(fd, 16) != 0) {
        close(fd);
        return NULL;
    }
    return net_wrap(fd);
}

NetSocket *net_connect(const char *host, int port) {
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    char service[16];
    snprintf(service, sizeof(service), "%d", port);
    struct addrinfo *addresses = NULL;
    if (getaddrinfo(host, service, &hints, &addresses) != 0) {
        return NULL;
    }
    int fd = -1;
    for (struct addrinfo *address = addresses; address; address = address->ai_next) {
        fd = socket(address->ai_family, address->ai_socktype, address->ai_protocol);
        if (fd >= 0 && connect(fd, address->ai_addr, address->ai_addrlen) == 0) {
            break;
        }
        if (fd >= 0) {
            close(fd);
        }
        fd = -1;
    }
    freeaddrinfo(addresses);
    return net_wrap(fd);
}

NetSocket *net_accept(NetSocket *listener) {
    if (!listener) {
        return NULL;
    }
    return net_wrap(accept(listener->fd, NULL, NULL));
}

int net_send(NetSocket *peer, const char *text) {
    if (!peer) {
        return -1;
    }
    size_t length = strlen(text);
    size_t sent = 0;
    while (sent < length) {
        ssize_t count = send(peer->fd, text + sent, length - sent, 0);
        if (count <= 0) {
            return -1;
        }
        sent += (size_t)count;
    }
    return (int)sent;
}

const char *net_receive(NetSocket *peer) {
    if (!peer) {
        return "";
    }
    char buffer[4096];
    ssize_t count = recv(peer->fd, buffer, sizeof(buffer), 0);
    if (count <= 0) {
        return "";
    }
    char *text = (char *)malloc((size_t)count + 1);
    if (!text) {
        return "";
    }
    memcpy(text, buffer, (size_t)count);
    text[count] = '\0';
    return text;
}

int net_port(NetSocket *peer) {
    if (!peer) {
        return 0;
    }
    struct sockaddr_in address;
    socklen_t length = sizeof(address);
    if (getsockname(peer->fd, (struct sockaddr *)&address, &length) != 0) {
        return 0;
    }
    return (int)ntohs(address.sin_port);
}

int net_is_open(NetSocket *peer) {
    return peer != NULL;
}

void net_destroy(NetSocket *peer) {
    if (!peer) {
        return;
    }
    close(peer->fd);
    free(peer);
}
