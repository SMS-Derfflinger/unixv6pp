//Video.h
#ifndef DIAGNOSE_H
#define DIAGNOSE_H

#include "libyrosstd/stdarg.h"
#include "libyrosstd/stdio.h"

extern "C" {
void _diagnose_trace_on();
void _diagnose_trace_off();
bool _diagnose_is_trace_on();
void _diagnose_write_cstr(const char* str);
void _diagnose_clear_screen();
}

class Diagnose
{
	/* static const member */
	static const unsigned int COLUMNS = 80;
	static const unsigned short COLOR = 0x0B00;	/* char in bright CYAN */
	static const unsigned int SCREEN_ROWS = 25;	/* full screen rows */

public:
	Diagnose();
	~Diagnose();

	static void TraceOn() {
        _diagnose_trace_on();
    }
	static void TraceOff() {
        _diagnose_trace_off();
    }

	static void Write(const char* fmt, ...) {
        if (false == _diagnose_is_trace_on())
	    {
		    return;
	    }

	    char buf[1024];
	    va_list args;
	    va_start(args, fmt);
	    vsprintf(buf, fmt, args);
	    va_end(args);
	    _diagnose_write_cstr(buf);
    }

	static void ClearScreen() {
        _diagnose_clear_screen();
    }
};

#endif
