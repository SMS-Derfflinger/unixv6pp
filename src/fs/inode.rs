use bitflags::bitflags;
use core::{fmt::Error, mem, slice};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct INodeFlag: u32 {
        const ILOCK  = 0x1;   // 索引节点上锁
        const IUPD   = 0x2;   // 内存inode被修改过，需要更新相应外存inode
        const IACC   = 0x4;   // 内存inode被访问过，需要修改最近一次访问时间
        const IMOUNT = 0x8;   // 内存inode用于挂载子文件系统
        const IWANT  = 0x10;  // 有进程正在等待该内存inode被解锁
        const ITEXT  = 0x20;  // 内存inode对应进程图像的正文段
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct INodeMode: u32 {
        const IALLOC = 0x8000;  // 文件被使用
        const IFMT   = 0x6000;  // 文件类型掩码
        const IFDIR  = 0x4000;  // 文件类型：目录文件
        const IFCHR  = 0x2000;  // 字符设备特殊类型文件
        const IFBLK  = 0x6000;  // 块设备特殊类型文件，为0表示常规数据文件
        const ILARG  = 0x1000;  // 文件长度类型：大型或巨型文件
        const ISUID  = 0x800;   // 执行时将有效用户ID修改为文件所有者UID
        const ISGID  = 0x400;   // 执行时将有效组ID修改为文件所有者GID
        const ISVTX  = 0x200;   // 使用后仍然位于交换区上的正文段
        const IREAD  = 0x100;   // 对文件的读权限
        const IWRITE = 0x80;    // 对文件的写权限
        const IEXEC  = 0x40;    // 对文件的执行权限
        // 组合权限
        const IRWXU  = Self::IREAD.bits() | Self::IWRITE.bits() | Self::IEXEC.bits();
        const IRWXG  = Self::IRWXU.bits() >> 3;
        const IRWXO  = Self::IRWXU.bits() >> 6;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevId(pub i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalBlock(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalBlock(pub u32);

const I_ADDR_SIZE: usize = 10;

pub struct Inode {
    pub i_flag: INodeFlag, // 状态标志位
    pub i_mode: INodeMode, // 文件工作方式信息

    pub i_count: i32, // 引用计数
    pub i_nlink: i32, // 文件联结计数（目录树中不同路径名的数量）

    pub i_dev: DevId,  // 外存inode所在存储设备的设备号
    pub i_number: i32, // 外存inode区中的编号

    pub i_uid: i16, // 文件所有者的用户标识数
    pub i_gid: i16, // 文件所有者的组标识数

    pub i_size: u64,                          // 文件大小（字节）
    pub i_addr: [PhysicalBlock; I_ADDR_SIZE], // 文件逻辑块号和物理块号转换的基本索引表

    pub i_lastr: i32, // 最近一次读取文件的逻辑块号（用于判断是否预读）
}

#[derive(Debug, PartialEq)]
pub enum BmapError {
    FileTooLarge, // EFBIG
    AllocFailed,
}

pub struct BmapResult {
    pub phyblk: PhysicalBlock,
    pub rablock: Option<PhysicalBlock>, // None 表示放弃预读
}

#[derive(Debug, PartialEq)]
pub enum OpenError {
    NoSuchDevice, // ENXIO
}

enum IndexKind {
    /// lbn 0–5，直接从 i_addr[lbn] 取物理块号
    Direct { slot: usize },
    /// lbn 6–261，i_addr[6/7] -> 一次间接表 -> 数据块
    Indirect {
        i_addr_slot: usize, // 6 或 7
        inner: usize,       // 在一次间接表中的下标
    },
    /// lbn 262–33031，i_addr[8/9] -> 二次间接表 -> 一次间接表 -> 数据块
    DoubleIndirect {
        i_addr_slot: usize, // 8 或 9
        mid: usize,         // 在二次间接表中的下标
        inner: usize,       // 在一次间接表中的下标
    },
}

impl IndexKind {
    fn from_lbn(lbn: usize) -> Result<Self, BmapError> {
        const N: usize = Inode::ADDRESS_PER_INDEX_BLOCK; // 128
        let small = Inode::SMALL_FILE_BLOCK; // 6
        let large = Inode::LARGE_FILE_BLOCK; // 262
        let huge = Inode::HUGE_FILE_BLOCK; // 33032

        if lbn >= huge {
            return Err(BmapError::FileTooLarge);
        }
        if lbn < small {
            Ok(IndexKind::Direct { slot: lbn })
        } else if lbn < large {
            Ok(IndexKind::Indirect {
                i_addr_slot: (lbn - small) / N + 6,
                inner: (lbn - small) % N,
            })
        } else {
            Ok(IndexKind::DoubleIndirect {
                i_addr_slot: (lbn - large) / (N * N) + 8,
                mid: ((lbn - large) / N) % N,
                inner: (lbn - large) % N,
            })
        }
    }
}

#[allow(unused)]
impl Inode {
    pub const BLOCK_SIZE: usize = 512;
    pub const ADDRESS_PER_INDEX_BLOCK: usize = Self::BLOCK_SIZE / size_of::<i32>();
    pub const SMALL_FILE_BLOCK: usize = 6;
    pub const LARGE_FILE_BLOCK: usize = 128 * 2 + 6;
    pub const HUGE_FILE_BLOCK: usize = 128 * 128 * 2 + 128 * 2 + 6;
    pub const PIPSIZ: usize = Self::SMALL_FILE_BLOCK * Self::BLOCK_SIZE;

    pub const fn new_const() -> Self {
        Self {
            i_flag: INodeFlag::empty(),
            i_mode: INodeMode::empty(),
            i_count: 0,
            i_nlink: 0,
            i_dev: DevId(-1),
            i_number: -1,
            i_uid: -1,
            i_gid: -1,
            i_size: 0,
            i_addr: [PhysicalBlock(0); I_ADDR_SIZE],
            i_lastr: -1,
        }
    }

    pub fn new() -> Self {
        Self {
            i_flag: INodeFlag::empty(),
            i_mode: INodeMode::empty(),
            i_count: Default::default(),
            i_nlink: Default::default(),
            i_dev: DevId(-1),
            i_number: -1,
            i_uid: -1,
            i_gid: -1,
            i_size: Default::default(),
            i_addr: [PhysicalBlock(0); I_ADDR_SIZE],
            i_lastr: -1,
        }
    }

    pub fn read_i(&mut self, m_count: usize, m_offset: usize) -> Result<(), Error> {
        // TODO: user args
        //let u = kernel::get_user();

        if m_count == 0 {
            return Ok(());
        }

        self.i_flag |= INodeFlag::IACC;

        // char dev
        if (self.i_mode & INodeMode::IFMT) == INodeMode::IFCHR {
            let dev = self.i_addr[0].0 as i16;
            let major = (dev >> 8) as u8;
            // TODO: dev manager
            //kernel::get_device_manager()
            //    .get_char_device(major)
            //    .read(dev);
            return Ok(());
        }

        let is_blk = (self.i_mode & INodeMode::IFMT) == INodeMode::IFBLK;

        // TODO: user error
        while
        /*u.u_error == KernelError::NoError &&*/
        m_count != 0 {
            let lbn = m_offset / Self::BLOCK_SIZE;
            let offset = m_offset % Self::BLOCK_SIZE;
            let mut nbytes = (Self::BLOCK_SIZE - offset).min(m_count);

            let (dev, bn, rablock) = if !is_blk {
                let remain = self.i_size.saturating_sub(m_offset as u64);
                if remain == 0 {
                    return Ok(());
                }
                nbytes = nbytes.min(remain as usize);

                // unsafe
                let bmap_result = self.bmap(LogicalBlock(lbn as u32)).unwrap();
                if bmap_result.phyblk == PhysicalBlock(0) {
                    return Ok(());
                }
                let rablock = bmap_result.rablock.unwrap_or(PhysicalBlock(0));
                (self.i_dev, bmap_result.phyblk, rablock)
            } else {
                let dev = self.i_addr[0].0 as i16;
                let bn = PhysicalBlock(lbn as u32);
                let ra = PhysicalBlock(lbn as u32 + 1);
                (DevId(dev), bn, ra)
            };

            // TODO: buffer manager
            let buf = if self.i_lastr + 1 == lbn as i32 {
                //kernel::buf_breada(dev, bn, rablock)
            } else {
                //kernel::buf_bread(dev, bn)
            };

            self.i_lastr = lbn as i32;

            // TODO: user args and io move
            /*let src = &buf.as_slice()[offset..offset + nbytes];
            u.io_param.copy_to_user(src);

            u.io_param.m_base += nbytes;
            u.io_param.m_offset += nbytes as u64;
            u.io_param.m_count -= nbytes as u64;*/
        }

        Ok(())
    }

    pub fn write_i(&mut self, m_count: usize, m_offset: usize) -> Result<(), Error> {
        self.i_flag |= INodeFlag::IACC | INodeFlag::IUPD;

        // char device
        if (self.i_mode & INodeMode::IFMT) == INodeMode::IFCHR {
            let dev = self.i_addr[0].0 as i16;
            let major = (dev >> 8) as u8;
            // TODO: dev manager
            // kernel::get_device_manager().get_char_device(major).write(dev);
            return Ok(());
        }

        if m_count == 0 {
            return Ok(());
        }

        let is_blk = (self.i_mode & INodeMode::IFMT) == INodeMode::IFBLK;
        let mut m_count = m_count;
        let mut m_offset = m_offset;

        while m_count != 0 {
            let lbn = m_offset / Self::BLOCK_SIZE;
            let offset = m_offset % Self::BLOCK_SIZE;
            let nbytes = (Self::BLOCK_SIZE - offset).min(m_count);

            let (_dev, _bn) = if !is_blk {
                let result = self.bmap(LogicalBlock(lbn as u32)).unwrap();
                if result.phyblk == PhysicalBlock(0) {
                    return Ok(());
                }
                (self.i_dev, result.phyblk)
            } else {
                (DevId(self.i_addr[0].0 as i16), PhysicalBlock(lbn as u32))
            };

            // TODO: buffer manager
            // let mut buf = if nbytes == Self::BLOCK_SIZE {
            //     kernel::buf_get_blk(_dev, _bn)
            // } else {
            //     kernel::buf_bread(_dev, _bn)
            // };

            // TODO: io move
            // let dst = &mut buf.as_slice_mut()[offset..offset + nbytes];
            // io.copy_from_user(dst);

            m_offset += nbytes;
            m_count -= nbytes;

            // TODO: buffer manager
            // if      has_error          { drop(buf);           }
            // else if m_offset % BLOCK_SIZE == 0 { buf.into_bawrite(); }
            // else                       { buf.into_bdwrite();  }

            if !is_blk && self.i_size < m_offset as u64 {
                self.i_size = m_offset as u64;
            }

            self.i_flag |= INodeFlag::IUPD;
        }

        Ok(())
    }

    // map lbn to pbn
    pub fn bmap(&mut self, lbn: LogicalBlock) -> Result<BmapResult, BmapError> {
        let lbn = lbn.0 as usize;

        match IndexKind::from_lbn(lbn)? {
            // small
            IndexKind::Direct { slot } => {
                let phy = self.get_or_alloc_direct(slot)?;

                let rablock = if slot <= 4 {
                    let next = self.i_addr[slot + 1].0;
                    if next != 0 {
                        Some(PhysicalBlock(next))
                    } else {
                        None
                    }
                } else {
                    None
                };

                Ok(BmapResult {
                    phyblk: phy,
                    rablock,
                })
            }

            // large
            IndexKind::Indirect { i_addr_slot, inner } => {
                let mut indirect = self.get_or_alloc_indirect_buf(i_addr_slot)?;

                let (phy, rablock) = Self::resolve_in_indirect(&mut indirect, inner, self.i_dev)?;

                Ok(BmapResult {
                    phyblk: phy,
                    rablock,
                })
            }

            // huge
            IndexKind::DoubleIndirect {
                i_addr_slot,
                mid,
                inner,
            } => {
                let mut first = self.get_or_alloc_indirect_buf(i_addr_slot)?;

                let phy2 = first.read_table()[mid];
                // TODO: buffer manager
                let mut second: BufHandle = if phy2 == 0 {
                    // TODO: fs
                    //let new_buf =
                    //    kernel::fs_alloc(self.i_dev).ok_or_else(|| BmapError::AllocFailed)?;
                    let new_buf = BufHandle(core::ptr::null_mut());
                    first.write_table()[mid] = new_buf.blkno().0 as i32;
                    first.into_bdwrite();
                    new_buf
                } else {
                    drop(first); // brelse
                                 // TODO: buffer manager
                                 //kernel::buf_bread(self.i_dev, PhysicalBlock(phy2 as u32))
                    BufHandle(core::ptr::null_mut())
                };

                let (phy, rablock) = Self::resolve_in_indirect(&mut second, inner, self.i_dev)?;

                Ok(BmapResult {
                    phyblk: phy,
                    rablock,
                })
            }
        }
    }

    /// 直接索引：返回物理块号，为 0 则按需分配
    fn get_or_alloc_direct(&mut self, slot: usize) -> Result<PhysicalBlock, BmapError> {
        let phy = self.i_addr[slot];
        if phy.0 != 0 {
            return Ok(PhysicalBlock(phy.0));
        }
        // TODO: fs
        //let buf = kernel::fs_alloc(self.i_dev).ok_or(BmapError::AllocFailed)?;
        let buf = BufHandle(core::ptr::null_mut());
        let blkno = buf.blkno();
        buf.into_bdwrite();
        self.i_addr[slot] = blkno;
        self.i_flag |= INodeFlag::IUPD;
        Ok(blkno)
    }

    /// 读出 i_addr[slot] 指向的间接块，若为 0 则分配新块
    fn get_or_alloc_indirect_buf(&mut self, slot: usize) -> Result<BufHandle, BmapError> {
        let blkno = self.i_addr[slot];
        if blkno.0 == 0 {
            // TODO: fs
            //let buf = kernel::fs_alloc(self.i_dev).ok_or(BmapError::AllocFailed)?;
            let buf = BufHandle(core::ptr::null_mut());
            self.i_addr[slot] = buf.blkno();
            self.i_flag |= INodeFlag::IUPD;
            Ok(buf)
        } else {
            // TODO: buffer manager
            //Ok(kernel::buf_bread(self.i_dev, blkno))
            Err(BmapError::AllocFailed)
        }
    }

    /// 在一次间接表中找到 inner 处的数据块号，按需分配，并计算预读块号。
    /// 消费 indirect（bdwrite 或 brelse）。
    fn resolve_in_indirect(
        indirect: &mut BufHandle,
        inner: usize,
        dev: DevId,
    ) -> Result<(PhysicalBlock, Option<PhysicalBlock>), BmapError> {
        let rablock = if inner + 1 < Self::ADDRESS_PER_INDEX_BLOCK {
            let next = indirect.read_table()[inner + 1];
            if next != 0 {
                Some(PhysicalBlock(next as u32))
            } else {
                None
            }
        } else {
            None
        };

        let phy = indirect.read_table()[inner];
        if phy == 0 {
            // TODO: fs
            //let data_buf = kernel::fs_alloc(dev).ok_or(BmapError::AllocFailed)?;
            let data_buf = BufHandle(core::ptr::null_mut());
            let blkno = data_buf.blkno();
            indirect.write_table()[inner] = blkno.0 as i32;
            data_buf.into_bdwrite();
            Ok((blkno, rablock))
        } else {
            Ok((PhysicalBlock(phy as u32), rablock))
        }
    }

    pub fn open_i(&self, mode: u32) -> Result<(), OpenError> {
        let dev = self.i_addr[0].0 as i16;
        let major = (dev >> 8) as u8;
        // TODO: dev manager
        //let dev_mgr = kernel::get_device_manager();

        match self.i_mode & INodeMode::IFMT {
            INodeMode::IFCHR => {
                // TODO: dev manager
                /*if major as usize >= dev_mgr.n_chr_dev() {
                    return Err(OpenError::NoSuchDevice);
                }
                dev_mgr.get_char_device(major).open(dev, mode);*/
            }
            INodeMode::IFBLK => {
                // TODO: dev manager
                /*if major as usize >= dev_mgr.n_blk_dev() {
                    return Err(OpenError::NoSuchDevice);
                }
                dev_mgr.get_block_device(major).open(dev, mode);*/
            }
            _ => {}
        }

        Ok(())
    }

    pub fn close_i(&self, mode: u32) {
        if self.i_count > 1 {
            return;
        }

        let dev = self.i_addr[0].0 as i16;
        let major = (dev >> 8) as u8;
        // TODO: dev manager
        //let dev_mgr = kernel::get_device_manager();

        match self.i_mode & INodeMode::IFMT {
            INodeMode::IFCHR => {
                //dev_mgr.get_char_device(major).close(dev, mode);
            }
            INodeMode::IFBLK => {
                //dev_mgr.get_block_device(major).close(dev, mode);
            }
            _ => {}
        }
    }

    pub fn i_update(&self, time: i32) {
        if !self.i_flag.intersects(INodeFlag::IUPD | INodeFlag::IACC) {
            return;
        }

        // TODO: fs
        //let fs = kernel::get_filesystem();
        //if fs.get_fs(self.i_dev).s_ronly != 0 {
        //    return;
        //}

        //let sector = FileSystem::INODE_ZONE_START_SECTOR
        //    + self.i_number as usize / FileSystem::INODE_NUMBER_PER_SECTOR;

        //let mut buf = kernel::buf_bread(self.i_dev, PhysicalBlock(sector as u32));

        let mut d_inode = DiskInode {
            d_mode: self.i_mode,
            d_nlink: self.i_nlink,
            d_uid: self.i_uid,
            d_gid: self.i_gid,
            d_size: self.i_size,
            d_addr: self.i_addr,
            d_atime: if self.i_flag.contains(INodeFlag::IACC) {
                time
            } else {
                0
            },
            d_mtime: if self.i_flag.contains(INodeFlag::IUPD) {
                time
            } else {
                0
            },
        };

        //let offset = (self.i_number as usize % FileSystem::INODE_NUMBER_PER_SECTOR)
        //    * size_of::<DiskInode>();
        //let dst = &mut buf.as_slice_mut()[offset..offset + size_of::<DiskInode>()];
        //let src = unsafe {
        //    slice::from_raw_parts(
        //        &d_inode as *const DiskInode as *const u8,
        //        size_of::<DiskInode>(),
        //    )
        //};
        //dst.copy_from_slice(src);

        // TODO: buffer manager
        //kernel::buf_bwrite(buf);
    }

    pub fn i_trunc(&mut self) {
        if self.i_mode.intersects(INodeMode::IFCHR | INodeMode::IFBLK) {
            return;
        }

        // TODO: fs
        //let fs = kernel::get_filesystem();

        for i in (0..10).rev() {
            let blk = self.i_addr[i];
            if blk == PhysicalBlock(0) {
                continue;
            }

            if i >= 6 {
                // TODO: buffer manager
                //let first_buf = kernel::buf_bread(self.i_dev, blk);
                //let first_table = *first_buf.read_table();

                //for j in (0..128).rev() {
                //    if first_table[j] == 0 {
                //        continue;
                //    }
                //    if i >= 8 {
                //        let second_buf =
                //            kernel::buf_bread(self.i_dev, PhysicalBlock(first_table[j] as u32));
                //        for k in (0..128).rev() {
                //            let b = second_buf.read_table()[k];
                //            if b != 0 {
                //                // TODO: fs
                //                //fs.free(self.i_dev, PhysicalBlock(b as u32));
                //            }
                //        }
                //    }
                //    // TODO: fs
                //    //fs.free(self.i_dev, PhysicalBlock(first_table[j] as u32));
                //}
            }

            // TODO: fs
            //fs.free(self.i_dev, blk);
            self.i_addr[i] = PhysicalBlock(0);
        }

        self.i_size = 0;
        self.i_flag |= INodeFlag::IUPD;
        self.i_mode &= !(INodeMode::ILARG | INodeMode::IRWXU | INodeMode::IRWXG | INodeMode::IRWXO);
        self.i_nlink = 1;
    }

    fn unlock_and_wake(&mut self) {
        self.i_flag.remove(INodeFlag::ILOCK);
        if self.i_flag.contains(INodeFlag::IWANT) {
            self.i_flag.remove(INodeFlag::IWANT);
            // TODO: wake up
            //kernel::get_process_manager().wake_up_all(self as *mut _ as usize);
        }
    }

    fn lock_with_priority(&mut self, priority: i32) {
        // TODO: user args
        //let u = kernel::get_user();
        while self.i_flag.contains(INodeFlag::ILOCK) {
            self.i_flag.insert(INodeFlag::IWANT);
            //u.u_procp.sleep(self as *mut _ as usize, priority);
        }
        self.i_flag.insert(INodeFlag::ILOCK);
    }

    pub fn nf_rele(&mut self) {
        self.unlock_and_wake();
    }

    pub fn nf_lock(&mut self) {
        self.lock_with_priority(0);
    }

    pub fn prele(&mut self) {
        self.unlock_and_wake();
    }

    pub fn plock(&mut self) {
        self.lock_with_priority(0);
    }

    pub fn clean(&mut self) {
        self.i_mode = INodeMode::empty();
        self.i_nlink = 0;
        self.i_uid = -1;
        self.i_gid = -1;
        self.i_size = 0;
        self.i_lastr = -1;
        self.i_addr = [PhysicalBlock(0); 10];
    }

    pub fn i_copy(&mut self, buf: &Buf, inumber: usize) {
        // TODO: fs
        //let offset = (inumber % FileSystem::INODE_NUMBER_PER_SECTOR) * size_of::<DiskInode>();
        let offset = 0;

        let src = &buf.as_slice()[offset..offset + size_of::<DiskInode>()];

        // SAFETY: DiskInode is #[repr(C)] Plain Old Data
        let d_inode: DiskInode =
            unsafe { core::ptr::read_unaligned(src.as_ptr() as *const DiskInode) };

        self.i_mode = d_inode.d_mode;
        self.i_nlink = d_inode.d_nlink;
        self.i_uid = d_inode.d_uid;
        self.i_gid = d_inode.d_gid;
        self.i_size = d_inode.d_size;
        self.i_addr = d_inode.d_addr;
    }
}

#[repr(C)]
pub struct DiskInode {
    pub d_mode: INodeMode,
    pub d_nlink: i32,
    pub d_uid: i16,
    pub d_gid: i16,
    pub d_size: u64,
    pub d_addr: [PhysicalBlock; 10],
    pub d_atime: i32,
    pub d_mtime: i32,
}

// buffer, consider move to other part
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BufFlag: u32 {
        const B_WRITE  = 0x1;  // 写操作
        const B_READ   = 0x2;  // 读操作
        const B_DONE   = 0x4;  // I/O操作结束
        const B_ERROR  = 0x8;  // I/O因出错而终止
        const B_BUSY   = 0x10; // 缓存正在使用中
        const B_WANTED = 0x20; // 有进程等待该缓存，清B_BUSY时需唤醒
        const B_ASYNC  = 0x40; // 异步I/O，不需等待结束
        const B_DELWRI = 0x80; // 延迟写
    }
}

pub struct Buf {
    pub b_flags: BufFlag,

    pub b_forw: *mut Buf,
    pub b_back: *mut Buf,
    pub av_forw: *mut Buf,
    pub av_back: *mut Buf,

    pub b_dev: i16,             // 高8位主设备号，低8位次设备号
    pub b_wcount: i32,          // 需传送的字节数
    pub b_addr: *mut u8,        // 所管理缓冲区的首地址
    pub b_blkno: PhysicalBlock, // 磁盘物理块号
    pub b_error: i32,           // I/O出错信息
    pub b_resid: i32,           // 出错时尚未传送的剩余字节数
}

impl Buf {
    pub const BLOCK_SIZE: usize = 512;

    pub fn read_table(&self) -> &[i32; 128] {
        unsafe { &*(self.b_addr as *const [i32; 128]) }
    }

    pub fn write_table(&mut self) -> &mut [i32; 128] {
        unsafe { &mut *(self.b_addr as *mut [i32; 128]) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.b_addr, Self::BLOCK_SIZE) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.b_addr, Self::BLOCK_SIZE) }
    }

    pub fn is_busy(&self) -> bool {
        self.b_flags.contains(BufFlag::B_BUSY)
    }

    pub fn is_done(&self) -> bool {
        self.b_flags.contains(BufFlag::B_DONE)
    }

    pub fn has_error(&self) -> bool {
        self.b_flags.contains(BufFlag::B_ERROR)
    }

    pub fn is_async(&self) -> bool {
        self.b_flags.contains(BufFlag::B_ASYNC)
    }

    pub fn is_delwri(&self) -> bool {
        self.b_flags.contains(BufFlag::B_DELWRI)
    }
}

pub struct BufHandle(pub *mut Buf);

impl BufHandle {
    pub fn blkno(&self) -> PhysicalBlock {
        PhysicalBlock(unsafe { (*self.0).b_blkno.0 })
    }
    pub fn read_table(&self) -> &[i32; 128] {
        unsafe { &*((*self.0).b_addr as *const [i32; 128]) }
    }
    pub fn write_table(&mut self) -> &mut [i32; 128] {
        unsafe { &mut *((*self.0).b_addr as *mut [i32; 128]) }
    }
    pub fn into_bdwrite(self) {
        let ptr = self.0;
        mem::forget(self);
        // TODO: buffer manager
        //kernel::buf_bdwrite(ptr);
    }
}

impl Drop for BufHandle {
    fn drop(&mut self) {
        // TODO: buffer manager
        //kernel::buf_brelse(self.0);
    }
}
