#ifndef FILE_H
#define FILE_H

#include "INode.h"

/*
 * 打开文件控制块File类。
 * 该结构记录了进程打开文件
 * 的读、写请求类型，文件读写位置等等。
 */
class File
{
public:
	/* Enumerate */
	enum FileFlags
	{
		FREAD = 0x1,			/* 读请求类型 */
		FWRITE = 0x2,			/* 写请求类型 */
		FPIPE = 0x4				/* 管道类型 */
	};

	/* Functions */
public:
	/* Constructors */
	File();
	/* Destructors */
	~File();


	/* Member */
	unsigned int f_flag;		/* 对打开文件的读、写操作要求 */
	int		f_count;			/* 当前引用该文件控制块的进程数量 */
	Inode*	f_inode;			/* 指向打开文件的内存Inode指针 */
	int		f_offset;			/* 文件读写位置指针 */
};

struct open_files;
extern "C" struct open_files* OpenFiles_clone(struct open_files const*);

template <typename T>
struct remove_reference { using type = T; };
template <typename T>
struct remove_reference<T&> { using type = T; };
template <typename T>
struct remove_reference<T&&> { using type = T; };

template <typename T>
constexpr typename remove_reference<T>::type&& move(T&& val) noexcept {
	return (typename remove_reference<T>::type&&)val;
}

template <typename T>
constexpr T&& forward(typename remove_reference<T>::type&& val) noexcept {
	return (T&&)val;
}

template <typename T>
constexpr T&& forward(typename remove_reference<T>::type& val) noexcept {
	return (T&&)val;
}

template <typename T, typename U>
constexpr T exchange(T& val, U&& new_val) noexcept {
	T ret_val = move(val);
	val = forward<U>(new_val);
	return ret_val;
}

/*
 * 进程打开文件描述符表(OpenFiles)的定义
 * 进程的u结构中包含OpenFiles类的一个对象，
 * 维护了当前进程的所有打开文件。
 */
class OpenFiles
{
	/* static members */
public:
	static constexpr int NOFILES = 15;	/* 进程允许打开的最大文件数 */

public:
	OpenFiles();

	constexpr OpenFiles(const OpenFiles& other) noexcept
		: impl(OpenFiles_clone(other.impl)) {  }

	constexpr OpenFiles(OpenFiles&& other) noexcept
		: impl(exchange(other.impl, nullptr)) { }

	OpenFiles& operator=(const OpenFiles& other) noexcept {
		exchange(*this, OpenFiles(other));
		return *this;
	}

	OpenFiles& operator=(OpenFiles&& other) noexcept {
		this->impl = exchange(other.impl, nullptr);
		return *this;
	}

	~OpenFiles();

	/* Functions */
public:
	/*
	 * @comment 进程请求打开文件时，在打开文件描述符表中分配一个空闲表项
	 */
	int AllocFreeSlot();

	/*
	 * @comment 根据用户系统调用提供的文件描述符参数fd，
	 * 找到对应的打开文件控制块File结构
	 */
	File* GetF(int fd);
	/*
	 * @comment 为已分配到的空闲描述符fd和已分配的打开文件表中
	 * 空闲File对象建立勾连关系
	 */
	void SetF(int fd, File* pFile);

	open_files* impl;
};

/*
 * 文件I/O的参数类
 * 对文件读、写时需用到的读、写偏移量、
 * 字节数以及目标区域首地址参数。
 */
class IOParameter
{
	/* Functions */
public:
	/* Constructors */
	IOParameter();
	/* Destructors */
	~IOParameter();

	/* Members */
public:
	unsigned char* m_Base;	/* 当前读、写用户目标区域的首地址 */
	int m_Offset;	/* 当前读、写文件的字节偏移量 */
	int m_Count;	/* 当前还剩余的读、写字节数量 */
};

#endif
