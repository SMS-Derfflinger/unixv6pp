#include "file.h"
#include "syscall.h"

/*
创建文件系统调用c库封装函数
name：创建路径的地址
mode：创建模式，需指定文件主，同组用户，其他用户的工作方式
返回值：成功则返回进程打开文件号，失败返回-1
*/
int creat(char* pathname, unsigned int mode)
{
	return __syscall_ret(__syscall2(8, (long) pathname, mode));
}

/*
打开文件系统调用c库封装函数
name：打开文件路径的地址
mode：打开文件模式，采用读、写还是读写的方式
返回值：成功则返回进程打开文件号，失败返回-1
*/
int open(char* pathname, unsigned int mode)
{
	return __syscall_ret(__syscall2(5, (long) pathname, mode));
}

int close(int fd)
{
	return __syscall_ret(__syscall1(6, fd));
}

/*
读文件系统调用c库封装函数
fd：打开进程打开文件号
ubuf：目的区首地址
nbytes：要求读出的字节数
返回值：读取的实际数目（字节）
*/
int read(int fd, char* buf, int nbytes)
{
	return __syscall_ret(__syscall3(3, fd, (long) buf, nbytes));
}

/*
写文件系统调用c库封装函数
fd：打开进程打开文件号
ubuf：信息源首地址
nbytes：写入字节数
返回值：成功返回写入的实际数目（字节）
*/
int write(int fd, char* buf, int nbytes)
{
	return __syscall_ret(__syscall3(4, fd, (long) buf, nbytes));
}

int pipe(int* fildes)
{
	return __syscall_ret(__syscall1(42, (long) fildes));
}

/*
搜索文件位置系统调用c库封装函数
fd:打开文件号
如果ptrname == 0，则读写位置设置为offset
如果ptrname == 1，则读写位置加offset（可正可负）
如果ptrname == 2，则读写位置调整为文件长度加offset
如果ptrname > 2，为3～5，意义同0~2，但长度单位从一个字节变为512个字节
返回值：成功返回1，失败返回-1
*/
int seek(int fd,unsigned int offset,unsigned int ptrname)
{
	return __syscall_ret(__syscall3(19, fd, offset, ptrname));
}

/*
复制file指针于进程打开文件表中系统调用c库封装函数
fd：进程打开打开文件号
返回值：复制的进程打开文件号
*/
int dup(int fd)
{
	return __syscall_ret(__syscall1(41, fd));
}

/*
得到进程打开文件inode信息系统调用
fd：打开文件号
statbuf: 目的地址
返回值：成功返回1，失败返回-1
*/
int fstat(int fd,unsigned long statbuf)
{
	return __syscall_ret(__syscall2(28, fd, statbuf));
}
/*
得到进程打开文件inode信息系统调用
pathname：指定文件路径
des: 目的地址
返回值：成功返回1，失败返回-1
*/
int stat(char* pathname,unsigned long statbuf)
{
	return __syscall_ret(__syscall2(18, (long) pathname, statbuf));
}
/*
改变文件访问模式系统调用c库封装函数
pathname：文件路径
mode：修改的模式
返回值：成功返回1，失败返回-1
*/
int chmod(char* pathname,unsigned int mode)
{
	return __syscall_ret(__syscall2(15, (long) pathname, mode));
}

/*
改变文件文件主号和文件同组号系统调用c库封装函数
pathname：文件路径
mode：修改的模式
返回值：成功返回1，失败返回-1
*/
int chown(char* pathname,short uid, short gid)
{
	return __syscall_ret(__syscall3(16, (long) pathname, uid, gid));
}

/*
增加文件的访问路径系统调用c库封装函数
pathname：文件路径指针
newPathname：新的文件路径指针
返回值：成功返回1，失败返回-1
*/
int link(char* pathname,char* newPathname)
{
	return __syscall_ret(__syscall2(9, (long) pathname, (long) newPathname));
}
/*
解除文件索引系统调用c库封装函数
pathname：要解除索引的文件路径
返回值：成功返回1，失败返回-1
*/
int unlink(char* pathname)
{
	return __syscall_ret(__syscall1(10, (long) pathname));
}
/*
改变当前目录系统调用c库封装函数
pathname：要改变到的路径指针
返回值：成功返回1，失败返回-1
*/
int chdir(char* pathname)
{
	return __syscall_ret(__syscall1(12, (long) pathname));
}
/*
建立特殊文件系统调用c库封装函数
pathname：路径的指针
mode：创建模式
dev：设备号
返回值：成功返回1，失败返回-1
*/
int mknod(char* pathname,unsigned int mode, int dev)
{
	return __syscall_ret(__syscall3(14, (long) pathname, mode, dev));
}
