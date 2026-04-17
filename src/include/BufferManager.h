#ifndef BUFFER_MANAGER_H
#define BUFFER_MANAGER_H

#include "Buf.h"
#include "DeviceManager.h"

extern "C" void buffer_manager_initialize();
extern "C" Buf* buffer_get_blk(short dev, int blkno);
extern "C" void buffer_brelse(Buf* bp);
extern "C" void buffer_io_wait(Buf* bp);
extern "C" void buffer_io_done(Buf* bp);
extern "C" Buf* buffer_bread(short dev, int blkno);
extern "C" Buf* buffer_breada(short dev, int blkno, int rablkno);
extern "C" void buffer_bwrite(Buf* bp);
extern "C" void buffer_bdwrite(Buf* bp);
extern "C" void buffer_bawrite(Buf* bp);
extern "C" void buffer_clr_buf(Buf* bp);
extern "C" void buffer_bflush(short dev);
extern "C" bool buffer_swap(int blkno, unsigned long addr, int count, unsigned int flag);
extern "C" Buf* buffer_get_swap_buf();
extern "C" Buf* buffer_get_b_free_list();

class BufferManager
{
public:
	static const int NBUF = 15;
	static const int BUFFER_SIZE = 512;

public:
	BufferManager() {}
	~BufferManager() {}
	
	void Initialize() {
        buffer_manager_initialize();
    }
	
	Buf* GetBlk(short dev, int blkno) {
        return buffer_get_blk(dev, blkno);
    }

	void Brelse(Buf* bp) {
        buffer_brelse(bp);
    }

	void IOWait(Buf* bp) {
        buffer_io_wait(bp);
    }

	void IODone(Buf* bp) {
        buffer_io_done(bp);
    }

	Buf* Bread(short dev, int blkno) {
        return buffer_bread(dev, blkno);
    }

	Buf* Breada(short adev, int blkno, int rablkno) {
        return buffer_breada(adev, blkno, rablkno);
    }

	void Bwrite(Buf* bp) {
        buffer_bwrite(bp);
    }

	void Bdwrite(Buf* bp) {
        buffer_bdwrite(bp);
    }

	void Bawrite(Buf* bp) {
        buffer_bawrite(bp);
    }

	void ClrBuf(Buf* bp) {
        buffer_clr_buf(bp);
    }

	void Bflush(short dev) {
        buffer_bflush(dev);
    }

	bool Swap(int blkno, unsigned long addr, int count, enum Buf::BufFlag flag) {
        return buffer_swap(blkno, addr, count, (unsigned int)flag);
    }

	Buf& GetSwapBuf() {
        return *buffer_get_swap_buf();
    }

	Buf& GetBFreeList() {
        return *buffer_get_b_free_list();
    }
};

#endif
