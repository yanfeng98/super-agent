#ifndef TEXT_H
#define TEXT_H
#include <stddef.h>
#include <stdbool.h>

size_t count_words(const char *s);
int vowels(const char *s);
bool is_palindrome(const char *s);
char *upper_copy(const char *s);
#endif
