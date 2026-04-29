#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum PosixError {
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    EEXIST = 17,
    EXDEV = 18,
    ENODEV = 19,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    ENFILE = 23,
    EMFILE = 24,
    ENOTTY = 25,
    EFBIG = 27,
    ENOSPC = 28,
    ESPIPE = 29,
    EROFS = 30,
    EMLINK = 31,
    EPIPE = 32,
    ERANGE = 34,
    ENOSYS = 38,
    ELOOP = 40,
    EOVERFLOW = 75,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGIOT = 6,
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,
    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGSTKFLT = 16,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,
    SIGTTIN = 21,
    SIGTTOU = 22,
    SIGURG = 23,
    SIGXCPU = 24,
    SIGXFSZ = 25,
    SIGVTALRM = 26,
    SIGPROF = 27,
    SIGWINCH = 28,
    SIGIO = 29,
    SIGPWR = 30,
    SIGSYS = 31,
}

pub const PSLEP: u32 = 90;

impl Signal {
    pub const SIGMAX: u32 = 32;
}

pub const SIGMAX: usize = Signal::SIGMAX as usize;

impl TryFrom<u32> for Signal {
    type Error = PosixError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 | Self::SIGMAX.. => Err(PosixError::EINVAL),
            sig => Ok(unsafe { core::mem::transmute(sig) })
        }
    }
}

pub mod fs_constants {
    pub const SECTOR_SIZE: usize = 512;

    pub const KB: usize = 1024;
    pub const MB: usize = 1024 * KB;

    pub const DISK_SIZE_MB: usize = 16;
    pub const TOTAL_DISK_SECTORS: usize = DISK_SIZE_MB * MB / SECTOR_SIZE;

    pub const MBR_SECTORS: usize = 1;
    pub const KERNEL_SECTORS: usize = 511;
    pub const SUPERBLOCK_SECTORS: usize = 2;
    pub const INODE_SECTORS: usize = 510;
    pub const SWAP_SECTORS: usize = 2048;
    pub const DATA_SECTORS: usize = TOTAL_DISK_SECTORS
        - MBR_SECTORS
        - KERNEL_SECTORS
        - SUPERBLOCK_SECTORS
        - INODE_SECTORS
        - SWAP_SECTORS;

    pub const MBR_SECTOR_OFF: usize = 0;
    pub const KERNEL_SECTOR_OFF: usize = MBR_SECTOR_OFF + MBR_SECTORS;
    pub const SUPERBLOCK_SECTOR_OFF: usize = KERNEL_SECTOR_OFF + KERNEL_SECTORS;
    pub const INODE_SECTOR_OFF: usize = SUPERBLOCK_SECTOR_OFF + SUPERBLOCK_SECTORS;
    pub const DATA_SECTOR_OFF: usize = INODE_SECTOR_OFF + INODE_SECTORS;
    pub const SWAP_SECTOR_OFF: usize = DATA_SECTOR_OFF + DATA_SECTORS;
    pub const PAST_LAST_SECTOR_OFF: usize = SWAP_SECTOR_OFF + SWAP_SECTORS;

    pub const DISK_INODE_SIZE: usize = 64;
    pub const NMOUNT: usize = 5;
    pub const ROOTINO: i32 = 1;
    pub const INODE_NUMBER_PER_SECTOR: usize = 8;

    pub const BLOCK_SIZE: usize = 512; // Inode and buffer
}

pub mod platform {
    pub const RAM_BASE: usize = 0x8000_0000;
    pub const KERNEL_LOAD_BASE: usize = 0x8020_0000;
    pub const KERNEL_VIRT_BASE: usize = 0xc000_0000;
    pub const KERNEL_MMIO_BASE: usize = 0xf000_0000;

    pub const UART0_PHYS_BASE: usize = 0x1000_0000;
    pub const PLIC_PHYS_BASE: usize = 0x0c00_0000;
    pub const VIRTIO_MMIO_PHYS_BASE: usize = 0x1000_1000;

    pub const UART0_BASE: usize = KERNEL_MMIO_BASE;
    pub const UART0_IRQ: usize = 10;

    pub const PLIC_BASE: usize = KERNEL_MMIO_BASE + 0x0100_0000;
    pub const VIRTIO_MMIO_BASE: usize =
        KERNEL_MMIO_BASE + (VIRTIO_MMIO_PHYS_BASE - UART0_PHYS_BASE);
    pub const VIRTIO_MMIO_STRIDE: usize = 0x1000;
    pub const VIRTIO_MMIO_COUNT: usize = 8;

    pub const CPU_FREQ_HZ: usize = 10_000_000;
}
