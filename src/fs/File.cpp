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

extern "C" struct open_files* OpenFiles_new(void);
extern "C" void OpenFiles_free(struct open_files*);

/*==============================class OpenFiles===================================*/
OpenFiles::OpenFiles() : impl(OpenFiles_new()) { }

OpenFiles::~OpenFiles()
{
	if (this->impl)
		OpenFiles_free(this->impl);
}

extern "C" int OpenFiles_alloc_free_slot(struct open_files* impl);

int OpenFiles::AllocFreeSlot()
{
	User& u = Kernel::Instance().GetUser();
	return u.u_ar0[User::EAX] = OpenFiles_alloc_free_slot(this->impl);
}

extern "C" File* OpenFiles_get_file(struct open_files* impl, int fd);
extern "C" void OpenFiles_set_file(struct open_files* impl, int fd, File*);

File* OpenFiles::GetF(int fd)
{
	User& u = Kernel::Instance().GetUser();

	return OpenFiles_get_file(this->impl, fd);
}

void OpenFiles::SetF(int fd, File* pFile)
{
        OpenFiles_set_file(this->impl, fd, pFile);
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

