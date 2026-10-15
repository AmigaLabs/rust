#include <errno.h>
#include <fcntl.h>
#include <unistd.h>

/* dup3() differs from dup2() in two ways: it fails when both descriptors are
 * equal, and it accepts O_CLOEXEC. Set the close-on-exec flag after dup2()
 * because clib4 has no atomic equivalent. */
int dup3(int oldfd, int newfd, int flags) {
    if (oldfd == newfd || (flags & ~O_CLOEXEC) != 0) {
        errno = EINVAL;
        return -1;
    }
    if (dup2(oldfd, newfd) < 0)
        return -1;
    if (flags & O_CLOEXEC) {
        if (fcntl(newfd, F_SETFD, FD_CLOEXEC) < 0) {
            int err = errno;
            close(newfd);
            errno = err;
            return -1;
        }
    }
    return newfd;
}
