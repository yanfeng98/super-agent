#include "checksum.h"

static const uint32_t MOD_ADLER = 65521;

uint32_t adler32(const char *data) {
    uint32_t a = 1, b = 0;
    if (!data) return 0;
    for (const unsigned char *p = (const unsigned char *)data; *p; p++) {
        a = (a + *p) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    return (b << 16) | a;
}

uint32_t fnv1a(const char *data) {
    uint32_t hash = 2166136261u;
    if (!data) return hash;
    for (const unsigned char *p = (const unsigned char *)data; *p; p++) {
        hash ^= *p;
        hash *= 16777619u;
    }
    return hash;
}

int clamp(int value, int low, int high) {
    if (value < low) return low;
    if (value > high) return high;
    return value;
}

long gcd(long a, long b) {
    if (a < 0) a = -a;
    if (b < 0) b = -b;
    while (b != 0) {
        long t = a % b;
        a = b;
        b = t;
    }
    return a;
}

double mean3(double a, double b, double c) {
    return (a + b + c) / 3.0;
}
