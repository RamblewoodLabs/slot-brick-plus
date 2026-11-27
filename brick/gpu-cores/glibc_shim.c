/* Binds a core built against glibc 2.31 to the glibc 2.17 symbols the devices' firmware has
 * (Slot's own build is glibc 2.25). Each newer symbol the core uses is defined here, hidden,
 * on top of the old one. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdarg.h>
#include <time.h>
#define HIDDEN __attribute__((visibility("hidden")))
#define OLD(ret, name, args) ret __old_##name args; __asm__(".symver __old_" #name "," #name "@GLIBC_2.17");
OLD(double, log, (double)) OLD(double, exp, (double)) OLD(double, pow, (double, double))
OLD(double, log2, (double)) OLD(double, exp2, (double))
OLD(float, logf, (float)) OLD(float, expf, (float)) OLD(float, powf, (float, float))
OLD(float, log2f, (float)) OLD(float, exp2f, (float))
OLD(int, fcntl, (int, int, ...))
HIDDEN double log(double x) { return __old_log(x); }
HIDDEN double exp(double x) { return __old_exp(x); }
HIDDEN double pow(double x, double y) { return __old_pow(x, y); }
HIDDEN double log2(double x) { return __old_log2(x); }
HIDDEN double exp2(double x) { return __old_exp2(x); }
HIDDEN float logf(float x) { return __old_logf(x); }
HIDDEN float expf(float x) { return __old_expf(x); }
HIDDEN float powf(float x, float y) { return __old_powf(x, y); }
HIDDEN float log2f(float x) { return __old_log2f(x); }
HIDDEN float exp2f(float x) { return __old_exp2f(x); }
HIDDEN int fcntl64(int fd, int cmd, ...) {
    va_list ap; va_start(ap, cmd); void *arg = va_arg(ap, void *); va_end(ap);
    return __old_fcntl(fd, cmd, arg);
}
/* glibc 2.30's wait on a chosen clock, as 2.17's wait on the realtime one. */
HIDDEN int pthread_cond_clockwait(pthread_cond_t *c, pthread_mutex_t *m, clockid_t clk, const struct timespec *at) {
    if (clk == CLOCK_REALTIME) return pthread_cond_timedwait(c, m, at);
    struct timespec now, rt, abs;
    clock_gettime(clk, &now); clock_gettime(CLOCK_REALTIME, &rt);
    long long ns = (at->tv_sec - now.tv_sec) * 1000000000LL + (at->tv_nsec - now.tv_nsec);
    if (ns < 0) ns = 0;
    ns += rt.tv_nsec;
    abs.tv_sec = rt.tv_sec + ns / 1000000000LL; abs.tv_nsec = ns % 1000000000LL;
    return pthread_cond_timedwait(c, m, &abs);
}
