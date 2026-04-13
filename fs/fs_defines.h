#ifndef __FS_FS_DEFINES_H__
#define __FS_FS_DEFINES_H__

#ifndef __cplusplus
#error "Don't use plain C..."
#endif

namespace fs {

const unsigned long SECTOR_SIZE = 512;

const unsigned long KB = 1024;
const unsigned long MB = 1024 * KB;

const unsigned long DISK_SIZE_MB = 16;
const unsigned long TOTAL_DISK_SECTORS = DISK_SIZE_MB * MB / SECTOR_SIZE;

const unsigned long MBR_SECTORS = 1;
const unsigned long KERNEL_SECTORS = 509;
const unsigned long SUPERBLOCK_SECTORS = 2;
const unsigned long INODE_SECTORS = 512;
const unsigned long SWAP_SECTORS = 2048;
const unsigned long DATA_SECTORS = TOTAL_DISK_SECTORS - MBR_SECTORS - KERNEL_SECTORS \
				   - SUPERBLOCK_SECTORS - INODE_SECTORS - SWAP_SECTORS;

const unsigned long MBR_SECTOR_OFF = 0;
const unsigned long KERNEL_SECTOR_OFF = MBR_SECTOR_OFF + MBR_SECTORS;
const unsigned long SUPERBLOCK_SECTOR_OFF = KERNEL_SECTOR_OFF + KERNEL_SECTORS;
const unsigned long INODE_SECTOR_OFF = SUPERBLOCK_SECTOR_OFF + SUPERBLOCK_SECTORS;
const unsigned long DATA_SECTOR_OFF = INODE_SECTOR_OFF + INODE_SECTORS;
const unsigned long SWAP_SECTOR_OFF = DATA_SECTOR_OFF + DATA_SECTORS;
const unsigned long PAST_LAST_SECTOR_OFF = SWAP_SECTOR_OFF + SWAP_SECTORS;

static_assert(PAST_LAST_SECTOR_OFF == TOTAL_DISK_SECTORS, "Not consistent disk size");

}

#endif
