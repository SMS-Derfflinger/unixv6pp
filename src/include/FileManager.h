#ifndef FILE_MANAGER_H
#define FILE_MANAGER_H

#include "OpenFileManager.h"
#include "File.h"

class FileManager;

extern "C" void FileManager_open();
extern "C" void FileManager_creat();
extern "C" void FileManager_open1(Inode*, int, int);
extern "C" void FileManager_close();
extern "C" void FileManager_seek();
extern "C" void FileManager_dup();
extern "C" void FileManager_fstat();
extern "C" void FileManager_stat();
extern "C" void FileManager_stat1(Inode*, unsigned long);
extern "C" void FileManager_read();
extern "C" void FileManager_write();
extern "C" void FileManager_rdwr(unsigned int mode);
extern "C" void FileManager_pipe();
extern "C" void FileManager_readp(File*);
extern "C" void FileManager_writep(File*);
extern "C" Inode* FileManager_namei(unsigned int mode);
extern "C" Inode* FileManager_maknode(unsigned int mode);
extern "C" void FileManager_writedir(Inode*);
extern "C" void FileManager_setcurdir(char* pathname);
extern "C" bool FileManager_access(Inode*, unsigned int mode);
extern "C" Inode* FileManager_owner();
extern "C" void FileManager_chmod();
extern "C" void FileManager_chown();
extern "C" void FileManager_chdir();
extern "C" void FileManager_link();
extern "C" void FileManager_unlink();
extern "C" void FileManager_mknod();

/*
 * 文件管理类(FileManager)
 * 封装了文件系统的各种系统调用在核心态下处理过程。
 */
class FileManager
{
public:
	enum DirectorySearchMode
	{
		OPEN = 0,
		CREATE = 1,
		DELETE = 2
	};

public:
	inline FileManager() {}
	inline ~FileManager() {}

	inline void Initialize() { }
	inline void Open() { FileManager_open(); }
	inline void Creat() { FileManager_creat(); }
	inline void Open1(Inode* pInode, int mode, int trf) { FileManager_open1(pInode, mode, trf); }
	inline void Close() { FileManager_close(); }
	inline void Seek() { FileManager_seek(); }
	inline void Dup() { FileManager_dup(); }
	inline void FStat() { FileManager_fstat(); }
	inline void Stat() { FileManager_stat(); }
	inline void Stat1(Inode* pInode, unsigned long statBuf) { FileManager_stat1(pInode, statBuf); }
	inline void Read() { FileManager_read(); }
	inline void Write() { FileManager_write(); }
	inline void Rdwr(enum File::FileFlags mode) { FileManager_rdwr((unsigned int)mode); }
	inline void Pipe() { FileManager_pipe(); }
	inline void ReadP(File* pFile) { FileManager_readp(pFile); }
	inline void WriteP(File* pFile) { FileManager_writep(pFile); }
	inline Inode* NameI(enum DirectorySearchMode mode) { return FileManager_namei((unsigned int)mode); }
	inline Inode* MakNode(unsigned int mode) { return FileManager_maknode(mode); }
	inline void WriteDir(Inode* pInode) { FileManager_writedir(pInode); }
	inline void SetCurDir(char* pathname) { FileManager_setcurdir(pathname); }
	inline int Access(Inode* pInode, unsigned int mode) { return FileManager_access(pInode, mode); }
	inline Inode* Owner() { return FileManager_owner(); }
	inline void ChMod() { FileManager_chmod(); }
	inline void ChOwn() { FileManager_chown(); }
	inline void ChDir() { FileManager_chdir(); }
	inline void Link() { FileManager_link(); }
	inline void UnLink() { FileManager_unlink(); }
	inline void MkNod() { FileManager_mknod(); }
};

class DirectoryEntry
{
public:
	static const int DIRSIZ = 28;

public:
	inline DirectoryEntry() : m_ino(0), m_name{0} {}
	inline ~DirectoryEntry() {}

public:
	int m_ino;
	char m_name[DIRSIZ];
};

#endif
