#include <ctype.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static int usage(void) {
    fputs("usage: tcli upper|len|div|words|copy|now\n", stderr);
    return 1;
}

static int parse(const char *s, long long *out) {
    char *end;
    errno = 0;
    *out = strtoll(s, &end, 10);
    return *s && !*end && errno == 0;
}

int main(int argc, char **argv) {
    if (argc < 2) return usage();
    const char *cmd = argv[1];
    int n = argc - 2;
    char **args = argv + 2;
    if (!strcmp(cmd, "upper")) {
        for (int i = 0; i < n; i++) {
            if (i) putchar(' ');
            for (const char *p = args[i]; *p; p++) putchar(toupper((unsigned char)*p));
        }
        putchar('\n');
    } else if (!strcmp(cmd, "len") && n == 1) {
        printf("%zu\n", strlen(args[0]));
    } else if (!strcmp(cmd, "div") && n == 2) {
        long long a, b;
        if (!parse(args[0], &a) || !parse(args[1], &b)) {
            fputs("error: not a number\n", stderr);
            return 2;
        }
        if (b == 0) {
            fputs("error: division by zero\n", stderr);
            return 2;
        }
        printf("%lld / %lld = %lld remainder %lld\n", a, b, a / b, a % b);
    } else if (!strcmp(cmd, "words")) {
        int c, in_word = 0;
        long count = 0;
        while ((c = getchar()) != EOF) {
            if (isspace(c)) {
                in_word = 0;
            } else if (!in_word) {
                in_word = 1;
                count++;
            }
        }
        printf("%ld\n", count);
    } else if (!strcmp(cmd, "copy") && n == 2) {
        FILE *in = fopen(args[0], "rb");
        if (!in) {
            fprintf(stderr, "error: cannot read %s\n", args[0]);
            return 1;
        }
        size_t len = 0, cap = 256;
        char *buf = malloc(cap);
        size_t got;
        while ((got = fread(buf + len, 1, cap - len, in)) > 0) {
            len += got;
            if (len == cap) buf = realloc(buf, cap *= 2);
        }
        fclose(in);
        FILE *out = fopen(args[1], "wb");
        if (!out) {
            fprintf(stderr, "error: cannot write %s\n", args[1]);
            free(buf);
            return 1;
        }
        for (size_t i = 0; i < len; i++) fputc(toupper((unsigned char)buf[i]), out);
        fclose(out);
        free(buf);
        printf("copied %zu bytes\n", len);
    } else if (!strcmp(cmd, "now")) {
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        printf("now: %lld%09ld\n", (long long)ts.tv_sec, ts.tv_nsec);
    } else {
        return usage();
    }
    return 0;
}
