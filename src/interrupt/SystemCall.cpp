#include "SystemCall.h"
#include "User.h"
#include "Kernel.h"

extern "C" {
unsigned int get_time();
void time_set(unsigned int value);
unsigned int time_tout();
void time_set_tout(unsigned int value);
unsigned long time_tout_address();
}

SystemCall::SystemCall()
{
	//nothing to do here
}

SystemCall::~SystemCall()
{
	//nothing to do here
}

void SystemCall::Trap1(int (*func)())
{
	User& u = Kernel::Instance().GetUser();

	User_get_intflg() = 1;
	func();
	User_get_intflg() = 0;
}

void SystemCall::Trap1ByNumber(unsigned int number)
{
	switch ( number )
	{
	case 31: Trap1(Sys_Stty); return;
	case 32: Trap1(Sys_Gtty); return;
	default:
		User_get_error() = User::ENOSYS;
		return;
	}
}

extern "C" void cpp_system_call_trap1(unsigned int number)
{
	SystemCall::Trap1ByNumber(number);
}

/*	31 = stty	count = 1	*/
int SystemCall::Sys_Stty()
{
    // TODO
	/*Inode* pInode;
	User& u = Kernel::Instance().GetUser();
	int fd = User_get_arg()[0];
	// TTy* pTTy = (TTy *)User_get_arg()[1];

	if ( (pInode = OpenFiles_get_inode(fd)) == NULL )
	{
		return 0;
	}
	if ( (pInode->i_mode & Inode::IFMT) != Inode::IFCHR )
	{
		User_get_error() = User::ENOTTY;
		return 0;
	}
	short dev = Inode_get_dev(pInode);
        g_ConsoleDevice.SgTTy(dev, pTTy);*/

	return 0;	/* GCC likes it ! */
}

/*	32 = gtty	count = 1	*/
int SystemCall::Sys_Gtty()
{
    // TODO
	/*Inode* pInode;
	User& u = Kernel::Instance().GetUser();
	int fd = User_get_arg()[0];
	// TTy* pTTy = (TTy *)User_get_arg()[1];

	if ( (pInode = OpenFiles_get_inode(fd)) == NULL )
	{
		return 0;
	}
	if ( (pInode->i_mode & Inode::IFMT) != Inode::IFCHR )
	{
		User_get_error() = User::ENOTTY;
		return 0;
	}
	short dev = Inode_get_dev(pInode);
        g_ConsoleDevice.SgTTy(dev, pTTy);*/

	return 0;	/* GCC likes it ! */
}
