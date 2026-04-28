#include "sys.h"
#include "stdlib.h"
#include "stdio.h"
#include "syscall.h"

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
int brk(void * newEndDataAddr)
{
	long res = __syscall1(17, (long) newEndDataAddr);
	if (res >= 0)
		return (int) res;
	errno = -1 * res;
	printf("%d\n", errno);
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

unsigned long fakeedata = 0;
void* sbrk(int increment)
{
	if (fakeedata == 0)
	{
		fakeedata = brk(0);
	}
	unsigned long newedata = fakeedata + increment - 1;
	brk((void*) (((newedata >> 12) + 1) << 12));
	fakeedata = newedata + 1;
	return (void*) fakeedata;
}
