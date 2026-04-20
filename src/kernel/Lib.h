#ifndef LIB_H
#define LIB_H

/* 系统调用的C库封装函数，为避免跟内核中函数重名，加上lib_前缀 */

extern "C" {
int _lib_creat(char* pathname, unsigned int mode);
int _lib_open(const char* pathname, unsigned int mode);
int _lib_close(int fd);
int _lib_write(int fd, char* buf, int nbytes);
int _lib_read(int fd, char* buf, int nbytes);
int _lib_exit(int status);
int _lib_wait(int* status);
int _lib_fork();
int _lib_pipe(int* fildes);
int _lib_execv(char* pathname, char* argv[]);
int _lib_seek(int fd, unsigned int offset, unsigned int ptrname);
int _lib_dup(int fd);
int _lib_fstat(int fd, unsigned long statbuf);
int _lib_stat(char* pathname, unsigned long statbuf);
int _lib_chmod(char* pathname, unsigned int mode);
int _lib_chown(char* pathname, short uid, short gid);
short _lib_getuid();
int _lib_setuid(short uid);
short _lib_getgid();
int _lib_setgid(short gid);
int _lib_getpid();
int _lib_nice(int change);
int _lib_sig(int signal, unsigned long func);
int _lib_kill(int pid, int signal);
int _lib_sleep(unsigned int seconds);
int _lib_pwd(char* pwd);
int _lib_brk(unsigned int newSize);
int _lib_link(char* pathname, char* newPathname);
int _lib_unlink(char* pathname);
int _lib_chdir(char* pathname);
int _lib_mknod(char* pathname, unsigned int mode, int dev);
int _lib_sync_file_system();
}

inline int lib_creat(char* pathname, unsigned int mode)
{
	return _lib_creat(pathname, mode);
}

inline int lib_open(const char* pathname, unsigned int mode)
{
	return _lib_open(pathname, mode);
}

inline int lib_close(int fd)
{
	return _lib_close(fd);
}

inline int lib_write(int fd, char* buf, int nbytes)
{
	return _lib_write(fd, buf, nbytes);
}

inline int lib_read(int fd, char* buf, int nbytes)
{
	return _lib_read(fd, buf, nbytes);
}

inline int lib_exit(int status)
{
	return _lib_exit(status);
}

inline int lib_wait(int* status)
{
	return _lib_wait(status);
}

inline int lib_fork()
{
	return _lib_fork();
}

inline int lib_pipe(int* fildes)
{
	return _lib_pipe(fildes);
}

inline int lib_execv(char* pathname, char* argv[])
{
	return _lib_execv(pathname, argv);
}

inline int lib_seek(int fd, unsigned int offset, unsigned int ptrname)
{
	return _lib_seek(fd, offset, ptrname);
}

inline int lib_dup(int fd)
{
	return _lib_dup(fd);
}

inline int lib_fstat(int fd, unsigned long statbuf)
{
	return _lib_fstat(fd, statbuf);
}

inline int lib_stat(char* pathname, unsigned long statbuf)
{
	return _lib_stat(pathname, statbuf);
}

inline int lib_chmod(char* pathname, unsigned int mode)
{
	return _lib_chmod(pathname, mode);
}

inline int lib_chown(char* pathname, short uid, short gid)
{
	return _lib_chown(pathname, uid, gid);
}

inline short lib_getuid()
{
	return _lib_getuid();
}

inline int lib_setuid(short uid)
{
	return _lib_setuid(uid);
}

inline short lib_getgid()
{
	return _lib_getgid();
}

inline int lib_setgid(short gid)
{
	return _lib_setgid(gid);
}

inline int lib_getpid()
{
	return _lib_getpid();
}

inline int lib_nice(int change)
{
	return _lib_nice(change);
}

inline int lib_sig(int signal, void (*func)())
{
	return _lib_sig(signal, (unsigned long)func);
}

inline int lib_kill(int pid, int signal)
{
	return _lib_kill(pid, signal);
}

inline int lib_sleep(unsigned int seconds)
{
	return _lib_sleep(seconds);
}

inline int lib_pwd(char* pwd)
{
	return _lib_pwd(pwd);
}

inline int lib_brk(unsigned int newSize)
{
	return _lib_brk(newSize);
}

inline int lib_link(char* pathname, char* newPathname)
{
	return _lib_link(pathname, newPathname);
}

inline int lib_unlink(char* pathname)
{
	return _lib_unlink(pathname);
}

inline int lib_chdir(char* pathname)
{
	return _lib_chdir(pathname);
}

inline int lib_mknod(char* pathname, unsigned int mode, int dev)
{
	return _lib_mknod(pathname, mode, dev);
}

inline int lib_syncFileSystem()
{
	return _lib_sync_file_system();
}

#endif
