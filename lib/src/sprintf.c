#ifdef _UNITTEST
#include "ctype.h"
#include "print_parse.h"
#include "stdio.h"
#include "string.h"
#include "valist.h"
#else
#include <ctype.h>
#include <print_parse.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <valist.h>
#endif

static int sprintf_char(char* buffer, va* pva)
{
	int ch = va_arg(*pva, int);
	*buffer = (char)ch;
	return 1;
}

static int sprintf_string(char* buffer, struct print_info* info, va* pva)
{
	char* pstr = va_arg(*pva, char*);
	int padding = 0;
	char* bp = buffer;
	int i;
	int strl;

	if (pstr == 0)
		pstr = "(null)";
	strl = strlen(pstr);
	if (info->width == -1) info->width = strl;
	if (info->prec == -1) info->prec = strl;
	if (info->prec > info->width) info->width = info->prec;
	padding = info->width - (strl > info->prec ? info->prec : strl);
	if (!info->left)
		while (bp < buffer + padding) *bp++ = ' ';
	for (i = 0; *pstr && i < info->width - padding; *bp++ = *pstr++, i++);
	if (info->left)
		while (bp < buffer + info->width) *bp++ = ' ';
	return info->width;
}

static int sprintf_interger(char* buffer, struct print_info* info, va* pva)
{
	char num[256];
	char* pn = buffer;
	char pre[20];
	char* bp = 0;
	char* bpre = pre;
	int strl = 0;
	int padding = 0;

	if (info->spec == 'i' || info->spec == 'd')
	{
		int i_num = va_arg(*pva, int);
		bp = itoa(i_num, num, 10);
	}
	else
	{
		unsigned int ui_num = va_arg(*pva, unsigned int);
		int radix = 10;
		switch (info->spec)
		{
		case 'x':
		case 'X':
			radix = 16;
			break;
		case 'o':
			radix = 8;
			break;
		case 'u':
		default:
			radix = 10;
			break;
		}
		bp = itoa(ui_num, num, radix);
	}

	if (!bp) return 0;
	if (info->alt)
	{
		switch (info->spec)
		{
		case 'x':
		case 'X':
			*bpre++ = '0';
			*bpre++ = info->spec;
			break;
		case 'o':
			*bpre++ = '0';
			break;
		}
	}
	if (info->showsign && *bp != '-' && (info->spec == 'd' || info->spec == 'i')) *bpre++ = '+';
	*bpre = 0;
	strl = strlen(bp) + strlen(bpre);
	if (info->width < strl) info->width = strl;
	padding = info->width - strl;
	if (!info->left)
		while (pn < buffer + padding) *pn++ = ' ';
	bpre = pre;
	while (*bpre && (*pn++ = *bpre++));
	while (*bp && (*pn++ = *bp++));
	if (info->left)
		while (pn < buffer + info->width) *pn++ = ' ';
	return info->width;
}

static int sprintf_double(char* buffer, struct print_info* info, va* pva)
{
	char num[256];
	char ex_num[256];
	char* pnum;
	char* bp = buffer;
	int strl = 0;
	int showplus = 0;
	int padding = 0;
	double dnum = va_arg(*pva, double);

	if (info->prec == -1) info->prec = 8;
	switch (info->spec)
	{
	case 'f':
	case 'F':
		pnum = lftoa(dnum, num, info->prec);
		if (pnum == 0) return 0;
		break;
	case 'e':
	case 'E':
		pnum = exlftoa(dnum, ex_num, info->prec, info->spec);
		if (pnum == 0) return 0;
		break;
	case 'g':
	case 'G':
		if (lftoa(dnum, num, info->prec) == 0) return 0;
		if (exlftoa(dnum, ex_num, info->prec, info->spec - 'G' + 'E') == 0) return 0;
		pnum = strlen(num) < strlen(ex_num) ? num : ex_num;
		break;
	default:
		return 0;
	}
	if (info->showsign && *pnum != '-') showplus = 1;
	strl = strlen(pnum) + showplus;
	if (info->width < strl) info->width = strl;
	padding = info->width - strl;
	if (!info->left)
		while (bp < buffer + padding) *bp++ = ' ';
	if (showplus) *bp++ = '+';
	while (*pnum && (*bp++ = *pnum++));
	if (info->left)
		while (bp < buffer + info->width) *bp++ = ' ';
	return info->width;
}

int _sprintf(char* buffer, char* fmt, va pva)
{
	struct print_spec spec;
	char* bp = buffer;

	if (buffer == 0) return -1;

	spec.fmt = spec.start_fmt = spec.end_fmt = fmt;
	while (find_spec(&spec) >= 0)
	{
		char* sbp = spec.end_fmt;
		while (sbp < spec.start_fmt) *bp++ = *sbp++;
		parse_spec(&spec);
		switch (spec.info.spec)
		{
		case 'c':
			bp += sprintf_char(bp, &pva);
			break;
		case 's':
			bp += sprintf_string(bp, &spec.info, &pva);
			break;
		case 'd':
		case 'i':
		case 'x':
		case 'X':
		case 'o':
		case 'u':
			bp += sprintf_interger(bp, &spec.info, &pva);
			break;
		case 'f':
		case 'F':
		case 'e':
		case 'E':
		case 'g':
		case 'G':
			bp += sprintf_double(bp, &spec.info, &pva);
			break;
		default:
			break;
		}
	}
	while (spec.end_fmt < spec.start_fmt) *bp++ = *spec.end_fmt++;
	*bp = 0;
	return bp - buffer;
}

int sprintf(char* buffer, char* fmt, ...)
{
	va pva;
	int ret;
	va_start(pva, fmt);
	ret = _sprintf(buffer, fmt, pva);
	va_end(pva);
	return ret;
}
