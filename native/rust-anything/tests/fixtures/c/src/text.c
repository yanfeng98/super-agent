#include <ctype.h>
#include <stdlib.h>
#include <string.h>
#include "text.h"

size_t count_words(const char *s) {
    size_t n = 0;
    int in_word = 0;
    if (!s) return 0;
    for (; *s; s++) {
        if (isspace((unsigned char)*s)) {
            in_word = 0;
        } else if (!in_word) {
            in_word = 1;
            n++;
        }
    }
    return n;
}

int vowels(const char *s) {
    int n = 0;
    if (!s) return -1;
    for (; *s; s++) {
        if (strchr("aeiouAEIOU", *s)) n++;
    }
    return n;
}

bool is_palindrome(const char *s) {
    if (!s) return false;
    size_t i = 0, j = strlen(s);
    while (i + 1 < j) {
        if (s[i] != s[j - 1]) return false;
        i++;
        j--;
    }
    return true;
}

/* Caller frees. */
char *upper_copy(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    for (size_t i = 0; i <= n; i++) out[i] = (char)toupper((unsigned char)s[i]);
    return out;
}
