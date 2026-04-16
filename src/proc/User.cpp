#include "User.h"
#include "Kernel.h"
#include "Utility.h"
#include "Video.h"
#include "libyrosstd/string.h"

void User::Pwd()
{
	strcpy(this->u_dirp, this->u_curdir);
}
