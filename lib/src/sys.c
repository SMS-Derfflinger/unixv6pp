#include "sys.h"
#include "stdlib.h"
#include "stdio.h"
#include "syscall.h"

typedef unsigned long uptr;

int execv(char *pathname, char *argv[])
{
	int argc = 0;
	while (argv[argc] != 0)
		argc++;
	return __syscall_ret(__syscall3(11, (long) pathname, argc, (long) argv));
}

int fork()
{
	return __syscall_ret(__syscall0(2));
}

int wait(int* status)	/* 获取子进程返回的Return Code */
{
	return __syscall_ret(__syscall1(7, (long) status));
}

int exit(int status)	/* 子进程返回给父进程的Return Code */
{
	return __syscall_ret(__syscall1(1, status));
}

int signal(int signal, void (*func)())
{
	return __syscall_ret(__syscall2(48, signal, (long) func));
}

int kill(int pid, int signal)
{
	return __syscall_ret(__syscall2(37, pid, signal));
}

int sleep(unsigned int seconds)
{
	return __syscall_ret(__syscall1(35, seconds));
}

/* 使用errno需要include "stdlib.h" */
static uptr cached_break = 0;

static uptr raw_brk(void *new_end_data_addr)
{
	long res = __syscall1(17, (long) new_end_data_addr);
	if (res >= 0)
		return (uptr) res;
	errno = -1 * res;
	return 0;
}

int brk(void * newEndDataAddr)
{
	uptr res = raw_brk(newEndDataAddr);
	if (res != 0)
	{
		if (newEndDataAddr != 0)
			cached_break = (uptr) newEndDataAddr;
		return 0;
	}
	return -1;
}

int syncFileSystem()
{
	return __syscall_ret(__syscall0(36));
}

int getPath(char *path)
{
	return __syscall_ret(__syscall1(39, (long) path));
}

int getpid()
{
	return __syscall_ret(__syscall0(20));
}

unsigned int getgid()
{
	long res = __syscall0(47);
	if (res >= 0)
		return (unsigned int) res;
	return -1;
}

unsigned int getuid()
{
	long res = __syscall0(24);
	if (res >= 0)
		return (unsigned int) res;
	return -1;
}

int setgid(short gid)
{
	return __syscall_ret(__syscall1(46, gid));
}

int setuid(short uid)
{
	return __syscall_ret(__syscall1(23, uid));
}

int gettime(struct tms* ptms)
{
	return __syscall_ret(__syscall1(13, (long) ptms));
}

int times(struct tms* ptms)
{
	return __syscall_ret(__syscall1(43, (long) ptms));
}

int getswtch()
{
	return __syscall_ret(__syscall0(38));
}

int trace(int lines)
{
	return __syscall_ret(__syscall1(29, lines));
}

void* sbrk(long increment)
{
	if (cached_break == 0)
	{
		cached_break = raw_brk(0);
		if (cached_break == 0)
			return (void*) -1;
	}

	uptr old_break = cached_break;
	if (increment == 0)
		return (void*) old_break;

	if (increment > 0 && (uptr) increment > ~0UL - old_break)
	{
		errno = 12;
		return (void*) -1;
	}
	if (increment < 0)
	{
		long available = (long) old_break;
		if (increment < -available)
		{
			errno = 12;
			return (void*) -1;
		}
	}

	uptr new_break = (uptr) ((long) old_break + increment);

	if (raw_brk((void*) new_break) == 0)
		return (void*) -1;

	cached_break = new_break;
	return (void*) old_break;
}
