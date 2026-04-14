#include "File.h"
#include "Kernel.h"

/*==============================class File===================================*/
File::File()
{
	this->f_count = 0;
	this->f_flag = 0;
	this->f_offset = 0;
	this->f_inode = NULL;
}

File::~File()
{
	//nothing to do here
}

extern "C" struct open_files* new_open_files(void);
extern "C" void free_open_files(struct open_files*);

/*==============================class OpenFiles===================================*/
OpenFiles::OpenFiles()
	: impl(new_open_files())
{
}

OpenFiles::~OpenFiles()
{
	free_open_files(this->impl);
}

extern "C" int ofiles_alloc_free_slot(struct open_files* impl, User::ErrorCode* perr);

int OpenFiles::AllocFreeSlot()
{
	User& u = Kernel::Instance().GetUser();

	return u.u_ar0[User::EAX] = ofiles_alloc_free_slot(this->impl, &u.u_error);
}

extern "C" File* ofiles_get_file(struct open_files* impl, int fd, User::ErrorCode* perr);

File* OpenFiles::GetF(int fd)
{
	User& u = Kernel::Instance().GetUser();

	return ofiles_get_file(this->impl, fd, &u.u_error);

	pFile = this->ProcessOpenFileTable[fd];
	if(pFile == NULL)
	{
		u.u_error = User::EBADF;
	}

	return pFile;	/* 即使pFile==NULL也返回它，由调用GetF的函数来判断返回值 */
}

void OpenFiles::SetF(int fd, File* pFile)
{
	if(fd < 0 || fd >= OpenFiles::NOFILES)
	{
		return;
	}
	/* 进程打开文件描述符指向系统打开文件表中相应的File结构 */
	this->ProcessOpenFileTable[fd] = pFile;
}

/*==============================class IOParameter===================================*/
IOParameter::IOParameter()
{
	this->m_Base = 0;
	this->m_Count = 0;
	this->m_Offset = 0;
}

IOParameter::~IOParameter()
{
	//nothing to do here
}

