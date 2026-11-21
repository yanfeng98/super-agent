#include <arpa/inet.h>
#include <ctype.h>
#include <errno.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#define TEXT "text/plain; charset=utf-8"
#define MAX_REQUEST 65536

static void write_all(int fd, const char *data, size_t len) {
    while (len > 0) {
        ssize_t n = write(fd, data, len);
        if (n <= 0) return;
        data += n;
        len -= (size_t)n;
    }
}

static void respond(int fd, int status, const char *type, const char *body, size_t len) {
    const char *reason = status == 200 ? "OK" : status == 400 ? "Bad Request" : "Not Found";
    char head[256];
    int n = snprintf(head, sizeof head,
                     "HTTP/1.1 %d %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nConnection: close\r\n\r\n",
                     status, reason, type, len);
    write_all(fd, head, (size_t)n);
    write_all(fd, body, len);
}

static void text(int fd, int status, const char *body) {
    respond(fd, status, TEXT, body, strlen(body));
}

static void unquote(char *s) {
    char *out = s;
    for (; *s; s++) {
        if (s[0] == '%' && isxdigit((unsigned char)s[1]) && isxdigit((unsigned char)s[2])) {
            char hex[3] = {s[1], s[2], 0};
            *out++ = (char)strtol(hex, NULL, 16);
            s += 2;
        } else {
            *out++ = *s;
        }
    }
    *out = 0;
}

static int parse(const char *s, long long *out) {
    char *end;
    errno = 0;
    *out = strtoll(s, &end, 10);
    return *s && !*end && errno == 0;
}

static void handle(int fd) {
    static char buf[MAX_REQUEST + 1];
    size_t len = 0;
    char *head_end = NULL;
    while (len < MAX_REQUEST) {
        ssize_t n = read(fd, buf + len, MAX_REQUEST - len);
        if (n <= 0) return;
        len += (size_t)n;
        buf[len] = 0;
        if ((head_end = strstr(buf, "\r\n\r\n"))) break;
    }
    if (!head_end) return;
    size_t content_length = 0;
    char *cl = strcasestr(buf, "\r\ncontent-length:");
    if (cl && cl < head_end) content_length = strtoul(cl + 17, NULL, 10);
    char *body = head_end + 4;
    while ((size_t)(buf + len - body) < content_length && len < MAX_REQUEST) {
        ssize_t n = read(fd, buf + len, MAX_REQUEST - len);
        if (n <= 0) break;
        len += (size_t)n;
    }
    size_t body_len = (size_t)(buf + len - body);
    if (body_len > content_length) body_len = content_length;

    char method[16] = {0}, target[4096] = {0};
    if (sscanf(buf, "%15s %4095s", method, target) != 2) return;
    char *query = strchr(target, '?');
    if (query) *query = 0;

    if (!strcmp(method, "GET") && !strcmp(target, "/hello")) {
        const char *json = "{\"message\":\"hello\"}";
        respond(fd, 200, "application/json", json, strlen(json));
    } else if (!strcmp(method, "POST") && !strcmp(target, "/upper")) {
        for (size_t i = 0; i < body_len; i++) body[i] = (char)toupper((unsigned char)body[i]);
        respond(fd, 200, TEXT, body, body_len);
    } else if (!strcmp(method, "GET") && !strncmp(target, "/div/", 5)) {
        char *a = target + 5;
        char *slash = strchr(a, '/');
        if (!slash || strchr(slash + 1, '/')) {
            text(fd, 404, "not found\n");
            return;
        }
        *slash = 0;
        char *b = slash + 1;
        unquote(a);
        unquote(b);
        long long x, y;
        if (!parse(a, &x) || !parse(b, &y)) {
            text(fd, 400, "error: not a number\n");
        } else if (y == 0) {
            text(fd, 400, "error: division by zero\n");
        } else {
            char out[256];
            snprintf(out, sizeof out, "%lld / %lld = %lld remainder %lld\n", x, y, x / y, x % y);
            text(fd, 200, out);
        }
    } else if (!strcmp(method, "GET") && !strcmp(target, "/now")) {
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        char out[64];
        snprintf(out, sizeof out, "now: %lld%09ld\n", (long long)ts.tv_sec, ts.tv_nsec);
        text(fd, 200, out);
    } else {
        text(fd, 404, "not found\n");
    }
}

int main(void) {
    signal(SIGPIPE, SIG_IGN);
    const char *port = getenv("PORT");
    int server = socket(AF_INET, SOCK_STREAM, 0);
    int yes = 1;
    setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof yes);
    struct sockaddr_in addr = {0};
    addr.sin_family = AF_INET;
    addr.sin_port = htons((unsigned short)atoi(port ? port : "8080"));
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    if (bind(server, (struct sockaddr *)&addr, sizeof addr) || listen(server, 16)) {
        perror("listen");
        return 1;
    }
    for (;;) {
        int client = accept(server, NULL, NULL);
        if (client < 0) continue;
        handle(client);
        close(client);
    }
}
