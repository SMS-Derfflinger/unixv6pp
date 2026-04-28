/*#ifdef _UNITTEST
#include "string.h"
#else
#include <string.h>
#endif
*/
#include "string.h"

int strcmp(unsigned char* dst, unsigned char* src)
{
    int ret = 0 ;
    while( ! (ret = *dst - *src) && *dst)
            ++src, ++dst;
    if ( ret < 0 )  
		ret = -1 ;
    else if ( ret > 0 )
		ret = 1 ;
    return( ret );	
}

char* strcpy(char* dst, char* src)
{
    char * cp = dst;
    while( (*(cp++) = *(src++)) )
		;
    return dst;
}

char* strcat(char* dst, char* src)
{
    char * cp = dst;
    while( *cp ) 
		++cp;           /* Find end of dst */
    while( (*(cp++) = *(src++)) );/* Copy src to end of dst */
    return dst ;
}

int strlen (char* str)
{
    int length = 0;
    while( *(str++) ) ++length;
	return( length );
}

void* memset(void* dst, int c, unsigned int len)
{
	char* s = (char *)dst;
	
	while (len-- != 0)
	{
		*s++ = (char)c;
	}
	return dst;
}

void memmove(unsigned int des, unsigned int src, unsigned int count)
{
	unsigned char* dst_ptr = (unsigned char*)des;
	unsigned char* src_ptr = (unsigned char*)src;

	if (dst_ptr == src_ptr || count == 0)
		return;

	if (dst_ptr < src_ptr)
	{
		while (count-- != 0)
		{
			*dst_ptr++ = *src_ptr++;
		}
	}
	else
	{
		dst_ptr += count;
		src_ptr += count;
		while (count-- != 0)
		{
			*--dst_ptr = *--src_ptr;
		}
	}
}

void memcpy(unsigned int des, unsigned int src, unsigned int count)
{
	unsigned char* dst_ptr = (unsigned char*)des;
	unsigned char* src_ptr = (unsigned char*)src;

	while (count-- != 0)
	{
		*dst_ptr++ = *src_ptr++;
	}
}


