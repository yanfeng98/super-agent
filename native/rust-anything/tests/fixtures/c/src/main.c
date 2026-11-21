#include <stdio.h>
#include "checksum.h"
#include "text.h"

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        printf("%s adler32=%08x fnv1a=%08x words=%zu\n", argv[i], adler32(argv[i]), fnv1a(argv[i]), count_words(argv[i]));
    }
    return 0;
}
