#ifndef FILE_H
#define FILE_H

#include "INode.h"

class File;

extern "C" int OpenFiles_alloc_free_slot();
extern "C" File* OpenFiles_get_file(int fd);
extern "C" Inode* OpenFiles_get_inode(int fd);
extern "C" void OpenFiles_set_file(int fd, File*);

/*
 * 进程打开文件描述符表(OpenFiles)的定义
 * 进程的u结构中包含OpenFiles类的一个对象，
 * 维护了当前进程的所有打开文件。
 */
class OpenFiles
{
public:
	static constexpr int NOFILES = 15;	/* 进程允许打开的最大文件数 */
};

/*
 * 文件I/O的参数类
 * 对文件读、写时需用到的读、写偏移量、
 * 字节数以及目标区域首地址参数。
 */
struct IOParameter
{
	unsigned char* m_Base;	/* 当前读、写用户目标区域的首地址 */
	int m_Offset;	/* 当前读、写文件的字节偏移量 */
	int m_Count;	/* 当前还剩余的读、写字节数量 */
};

#endif
