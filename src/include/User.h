#ifndef USER_H
#define USER_H

#include "MemoryDescriptor.h"
#include "Process.h"
#include "File.h"
#include "INode.h"
#include "FileManager.h"
#include "libyrosstd/string.h"

/*
 * @comment 该类与Unixv6中 struct user结构对应，因此只改变
 * 类名，不修改成员结构名字，关于数据类型的对应关系如下:
 */
class User
{
public:
	static const int EAX = 0;	/* User_get_ar0()[EAX]；访问现场保护区中EAX寄存器的偏移量 */

	/* u_error's Error Code */
	/* 1~32 来自linux 的内核代码中的/usr/include/asm/errno.h, 其余for V6++ */
	enum ErrorCode
	{
		NOERROR	= 0,	/* No error */
		EPERM	= 1,	/* Operation not permitted */
		ENOENT	= 2,	/* No such file or directory */
		ESRCH	= 3,	/* No such process */
		EINTR	= 4,	/* Interrupted system call */
		EIO		= 5,	/* I/O error */
		ENXIO	= 6,	/* No such device or address */
		E2BIG	= 7,	/* Arg list too long */
		ENOEXEC	= 8,	/* Exec format error */
		EBADF	= 9,	/* Bad file number */
		ECHILD	= 10,	/* No child processes */
		EAGAIN	= 11,	/* Try again */
		ENOMEM	= 12,	/* Out of memory */
		EACCES	= 13,	/* Permission denied */
		EFAULT  = 14,	/* Bad address */
		ENOTBLK	= 15,	/* Block device required */
		EBUSY 	= 16,	/* Device or resource busy */
		EEXIST	= 17,	/* File exists */
		EXDEV	= 18,	/* Cross-device link */
		ENODEV	= 19,	/* No such device */
		ENOTDIR	= 20,	/* Not a directory */
		EISDIR	= 21,	/* Is a directory */
		EINVAL	= 22,	/* Invalid argument */
		ENFILE	= 23,	/* File table overflow */
		EMFILE	= 24,	/* Too many open files */
		ENOTTY	= 25,	/* Not a typewriter(terminal) */
		ETXTBSY	= 26,	/* Text file busy */
		EFBIG	= 27,	/* File too large */
		ENOSPC	= 28,	/* No space left on device */
		ESPIPE	= 29,	/* Illegal seek */
		EROFS	= 30,	/* Read-only file system */
		EMLINK	= 31,	/* Too many links */
		EPIPE	= 32,	/* Broken pipe */
		ENOSYS	= 100,
		//EFAULT	= 106
	};

	static const int NSIG = 32;	/* 信号个数 */

	/* p_sig中接受到的信号定义 */
	static const int SIGNUL = 0;	/* No Signal Received */
	static const int SIGHUP = 1;	/* Hangup (kill controlling terminal) */
	static const int SIGINT = 2;    /* Interrupt from keyboard */
	static const int SIGQUIT = 3;	/* Quit from keyboard */
	static const int SIGILL = 4;	/* Illegal instrution */
	static const int SIGTRAP = 5;	/* Trace trap */
	static const int SIGABRT = 6;	/* use abort() API */
	static const int SIGBUS = 7;	/* Bus error */
	static const int SIGFPE = 8;	/* Floating point exception */
	static const int SIGKILL = 9;	/* Kill(can't be caught or ignored) */
	static const int SIGUSR1 = 10;	/* User defined signal 1 */
	static const int SIGSEGV = 11;	/* Invalid memory segment access */
	static const int SIGUSR2 = 12;	/* User defined signal 2 */
	static const int SIGPIPE = 13;	/* Write on a pipe with no reader, Broken pipe */
	static const int SIGALRM = 14;	/* Alarm clock */
	static const int SIGTERM = 15;	/* Termination */
	static const int SIGSTKFLT = 16; /* Stack fault */
	static const int SIGCHLD = 17; /* Child process has stopped or exited, changed */
	static const int SIGCONT = 18; /* Continue executing, if stopped */
	static const int SIGSTOP = 19; /* Stop executing */
	static const int SIGTSTP = 20; /* Terminal stop signal */
	static const int SIGTTIN = 21; /* Background process trying to read, from TTY */
	static const int SIGTTOU = 22; /* Background process trying to write, to TTY */
	static const int SIGURG = 23;  /* Urgent condition on socket */
	static const int SIGXCPU = 24; /* CPU limit exceeded */
	static const int SIGXFSZ = 25; /* File size limit exceeded */
	static const int SIGVTALRM = 26; /* Virtual alarm clock */
	static const int SIGPROF = 27; /* Profiling alarm clock */
	static const int SIGWINCH = 28; /* Window size change */
	static const int SIGIO = 29; /* I/O now possible */
	static const int SIGPWR = 30; /* Power failure restart */
	static const int SIGSYS = 31; /* invalid sys call */

public:
	/* Member Functions */
public:
	/* 检查当前用户是否是超级用户 */
	inline bool SUser() { return true; }
};

extern "C" unsigned long (*User_get_rsav_())[2];
inline unsigned long (&User_get_rsav())[2] {
	auto& ref = *User_get_rsav_();
	return ref;
}

extern "C" unsigned long (*User_get_ssav_())[2];
inline unsigned long (&User_get_ssav())[2] {
	auto& ref = *User_get_ssav_();
	return ref;
}

extern "C" Process* *User_get_procp_();
inline Process* &User_get_procp() {
	auto& ref = *User_get_procp_();
	return ref;
}

extern "C" MemoryDescriptor *User_get_MemoryDescriptor_();
inline MemoryDescriptor &User_get_MemoryDescriptor() {
	auto& ref = *User_get_MemoryDescriptor_();
	return ref;
}

extern "C" unsigned int* *User_get_ar0_();
inline unsigned int* &User_get_ar0() {
	auto& ref = *User_get_ar0_();
	return ref;
}

extern "C" int (*User_get_arg_())[5];
inline int (&User_get_arg())[5] {
	auto& ref = *User_get_arg_();
	return ref;
}

extern "C" char* *User_get_dirp_();
inline char* &User_get_dirp() {
	auto& ref = *User_get_dirp_();
	return ref;
}

extern "C" int *User_get_utime_();
inline int &User_get_utime() {
	auto& ref = *User_get_utime_();
	return ref;
}

extern "C" int *User_get_stime_();
inline int &User_get_stime() {
	auto& ref = *User_get_stime_();
	return ref;
}

extern "C" int *User_get_cutime_();
inline int &User_get_cutime() {
	auto& ref = *User_get_cutime_();
	return ref;
}

extern "C" int *User_get_cstime_();
inline int &User_get_cstime() {
	auto& ref = *User_get_cstime_();
	return ref;
}

extern "C" unsigned long (*User_get_signal_())[User::NSIG];
inline unsigned long (&User_get_signal())[User::NSIG] {
	auto& ref = *User_get_signal_();
	return ref;
}

extern "C" unsigned long (*User_get_qsav_())[2];
inline unsigned long (&User_get_qsav())[2] {
	auto& ref = *User_get_qsav_();
	return ref;
}

extern "C" bool *User_get_intflg_();
inline bool &User_get_intflg() {
	auto& ref = *User_get_intflg_();
	return ref;
}

extern "C" Inode* *User_get_cdir_();
inline Inode* &User_get_cdir() {
	auto& ref = *User_get_cdir_();
	return ref;
}

extern "C" Inode* *User_get_pdir_();
inline Inode* &User_get_pdir() {
	auto& ref = *User_get_pdir_();
	return ref;
}

extern "C" DirectoryEntry *User_get_dent_();
inline DirectoryEntry &User_get_dent() {
	auto& ref = *User_get_dent_();
	return ref;
}

extern "C" char (*User_get_dbuf_())[DirectoryEntry::DIRSIZ];
inline char (&User_get_dbuf())[DirectoryEntry::DIRSIZ] {
	auto& ref = *User_get_dbuf_();
	return ref;
}

extern "C" char (*User_get_curdir_())[128];
inline char (&User_get_curdir())[128] {
	auto& ref = *User_get_curdir_();
	return ref;
}

extern "C" User::ErrorCode *User_get_error_();
inline User::ErrorCode &User_get_error() {
	auto& ref = *User_get_error_();
	return ref;
}

extern "C" int *User_get_segflg_();
inline int &User_get_segflg() {
	auto& ref = *User_get_segflg_();
	return ref;
}

extern "C" short *User_get_uid_();
inline short &User_get_uid() {
	auto& ref = *User_get_uid_();
	return ref;
}

extern "C" short *User_get_gid_();
inline short &User_get_gid() {
	auto& ref = *User_get_gid_();
	return ref;
}

extern "C" short *User_get_ruid_();
inline short &User_get_ruid() {
	auto& ref = *User_get_ruid_();
	return ref;
}

extern "C" short *User_get_rgid_();
inline short &User_get_rgid() {
	auto& ref = *User_get_rgid_();
	return ref;
}

extern "C" IOParameter *User_get_IOParam_();
inline IOParameter &User_get_IOParam() {
	auto& ref = *User_get_IOParam_();
	return ref;
}

#endif

