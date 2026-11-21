#ifndef CHECKSUM_H
#define CHECKSUM_H
#include <stddef.h>
#include <stdint.h>

uint32_t adler32(const char *data);
uint32_t fnv1a(const char *data);
int clamp(int value, int low, int high);
long gcd(long a, long b);
double mean3(double a, double b, double c);
#endif
