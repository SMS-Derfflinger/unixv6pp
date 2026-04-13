/*
 * 宏定义：磁盘硬件信息。
 * 2051565 龚天遥
 * 创建于 2022年7月29日。
 */

#pragma once

#include <fs_defines.h>

/**
 * 硬盘硬件信息。
 */
class MachineProps {
public:
    /** 盘块尺寸（字节）。 */
    static const int BLOCK_SIZE = fs::SECTOR_SIZE;

    /** 启动引导程序 bootloader 占用块数。 */
    static const int BOOT_LOADER_BLOCKS = fs::MBR_SECTORS;

    /** Super Block 区占用块数。 */
    static const int SUPER_BLOCK_ZONE_BLOCKS = fs::SUPERBLOCK_SECTORS;

    /** Inode 区占用块数。 */
    static const int INODE_ZONE_BLOCKS = fs::INODE_SECTORS;

    /** 交换区占用块数。 */
    static const int SWAP_ZONE_BLOCKS = fs::SWAP_SECTORS;

    /** 内核映像文件区占用块数。不含 bootloader。 */
    static const int KERNEL_BIN_BLOCKS = fs::KERNEL_SECTORS;

    /** 内核映像文件与启动引导区占用总块数。 */
    static const int KERNEL_AND_BOOT_BLOCKS = BOOT_LOADER_BLOCKS + KERNEL_BIN_BLOCKS;

    /** 推荐的硬盘大小。 */
    static inline unsigned long long diskSize() {
	    return fs::TOTAL_DISK_SECTORS * fs::SECTOR_SIZE;
    }

private:

    // 禁止构造对象。

    MachineProps() {}
    MachineProps(const MachineProps&) {}
};
