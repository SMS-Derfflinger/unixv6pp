#ifndef BUFFER_MANAGER_H
#define BUFFER_MANAGER_H

#include "Buf.h"
#include "DeviceManager.h"

extern "C" void rust_buffer_manager_initialize();
extern "C" Buf* rust_buffer_get_blk(short dev, int blkno);
extern "C" void rust_buffer_brelse(Buf* bp);
extern "C" void rust_buffer_io_wait(Buf* bp);
extern "C" void rust_buffer_io_done(Buf* bp);
extern "C" Buf* rust_buffer_bread(short dev, int blkno);
extern "C" Buf* rust_buffer_breada(short dev, int blkno, int rablkno);
extern "C" void rust_buffer_bwrite(Buf* bp);
extern "C" void rust_buffer_bdwrite(Buf* bp);
extern "C" void rust_buffer_bawrite(Buf* bp);
extern "C" void rust_buffer_clr_buf(Buf* bp);
extern "C" void rust_buffer_bflush(short dev);
extern "C" bool rust_buffer_swap(int blkno, unsigned long addr, int count, unsigned int flag);
extern "C" Buf* rust_buffer_get_swap_buf();
extern "C" Buf* rust_buffer_get_b_free_list();

class BufferManager
{
public:
	static const int NBUF = 15;
	static const int BUFFER_SIZE = 512;

public:
	BufferManager() {}
	~BufferManager() {}
	
	void Initialize() {
        rust_buffer_manager_initialize();
    }
	
	Buf* GetBlk(short dev, int blkno) {
        return rust_buffer_get_blk(dev, blkno);
    }

	void Brelse(Buf* bp) {
        rust_buffer_brelse(bp);
    }

	void IOWait(Buf* bp) {
        rust_buffer_io_wait(bp);
    }

	void IODone(Buf* bp) {
        rust_buffer_io_done(bp);
    }

	Buf* Bread(short dev, int blkno) {
        return rust_buffer_bread(dev, blkno);
    }

	Buf* Breada(short adev, int blkno, int rablkno) {
        return rust_buffer_breada(adev, blkno, rablkno);
    }

	void Bwrite(Buf* bp) {
        rust_buffer_bwrite(bp);
    }

	void Bdwrite(Buf* bp) {
        rust_buffer_bdwrite(bp);
    }

	void Bawrite(Buf* bp) {
        rust_buffer_bawrite(bp);
    }

	void ClrBuf(Buf* bp) {
        rust_buffer_clr_buf(bp);
    }

	void Bflush(short dev) {
        rust_buffer_bflush(dev);
    }

	bool Swap(int blkno, unsigned long addr, int count, enum Buf::BufFlag flag) {
        return rust_buffer_swap(blkno, addr, count, (unsigned int)flag);
    }

	Buf& GetSwapBuf() {
        return *rust_buffer_get_swap_buf();
    }

	Buf& GetBFreeList() {
        return *rust_buffer_get_b_free_list();
    }
};

#endif
