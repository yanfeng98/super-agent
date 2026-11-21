#include <assert.h>
#include "checksum.h"

int main(void) {
    assert(adler32("Wikipedia") == 0x11E60398);
    assert(gcd(12, 18) == 6);
    return 0;
}
