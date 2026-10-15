#include <errno.h>
#include <stdio.h>
#include <sys/types.h>
#include <sys/uio.h>
#include <unistd.h>

/* clib4 has writev() and lseek() but no positional variant, so move the file
 * offset, write, and restore it. Other users of the same descriptor can observe
 * the temporary offset. */
ssize_t pwritev(int fd, const struct iovec *iov, int iovcnt, off_t offset) {
    off_t saved = lseek(fd, 0, SEEK_CUR);
    if (saved < 0)
        return -1;
    if (lseek(fd, offset, SEEK_SET) < 0)
        return -1;
    ssize_t n = writev(fd, iov, iovcnt);
    int err = errno;
    lseek(fd, saved, SEEK_SET);
    errno = err;
    return n;
}
