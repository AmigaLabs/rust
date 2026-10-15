#include <stddef.h>

/* madvise() only passes a hint about the access pattern, and clib4 has no
 * equivalent. Accepting the hint and doing nothing is correct. */
int madvise(void *addr, size_t length, int advice) {
    (void) addr;
    (void) length;
    (void) advice;
    return 0;
}
