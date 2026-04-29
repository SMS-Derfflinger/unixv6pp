use core::ptr::NonNull;

use bitflags::bitflags;
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListAtomicLink, UnsafeRef};

use crate::{constants::fs_constants, dev::buffer_manager::global_buffer_manager};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevId(pub i16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalBlock(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalBlock(pub u32);

bitflags! {
    #[repr(transparent)]
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

impl PhysicalBlock {
    pub const ZERO: Self = Self(0);

    pub const fn new(blkid: u32) -> Option<Self> {
        if blkid != 0 {
            Some(Self(blkid))
        } else {
            None
        }
    }
}

/// [`Buffer`] holds a reference to the underlying buffer.
///
/// Dropping the [`Buffer`] calls `bm.release(buf)`.
pub struct Buffer {
    bp: BufRef,
}

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Buffer {
    pub(crate) const fn new(bp: BufRef) -> Self {
        Self { bp }
    }

    fn deref(&self) -> &Buf {
        self.bp.as_ref()
    }

    fn deref_mut(&mut self) -> &mut Buf {
        self.bp.as_mut()
    }

    pub fn as_slice_mut<T: Copy>(&mut self) -> &mut [T] {
        let bytes = self.deref_mut().as_slice_mut();
        let addr = bytes.as_mut_ptr() as *mut T;
        let len_in_bytes = bytes.len();

        assert!(addr.is_aligned(), "Unaligned pointer");
        assert!(len_in_bytes % size_of::<T>() == 0, "Wrong type");

        unsafe { core::slice::from_raw_parts_mut(addr as *mut T, len_in_bytes / size_of::<T>()) }
    }

    pub fn as_slice<T: Copy>(&self) -> &[T] {
        let bytes = self.deref().as_slice();
        let addr = bytes.as_ptr() as *const T;
        let len_in_bytes = bytes.len();

        assert!(addr.is_aligned(), "Unaligned pointer");
        assert!(len_in_bytes % size_of::<T>() == 0, "Wrong type");

        unsafe { core::slice::from_raw_parts(addr, len_in_bytes / size_of::<T>()) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_slice::<u8>()
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.as_slice_mut::<u8>()
    }

    pub fn phyblk(&self) -> PhysicalBlock {
        self.deref().b_blkno
    }

    pub(crate) fn into_ref(self) -> BufRef {
        let bp = self.bp;
        core::mem::forget(self);
        bp
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        global_buffer_manager().brelse(self.bp);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufRef(NonNull<Buf>);

impl BufRef {
    pub fn from_mut(buf: &mut Buf) -> Self {
        Self(NonNull::from(buf))
    }

    pub fn from_ref(buf: &Buf) -> Self {
        Self(NonNull::from(buf))
    }

    pub fn from_unsafe_ref(buf: UnsafeRef<Buf>) -> Self {
        unsafe { Self(NonNull::new_unchecked(UnsafeRef::into_raw(buf))) }
    }

    pub fn as_ref(self) -> &'static Buf {
        unsafe { self.0.as_ref() }
    }

    pub fn as_mut(mut self) -> &'static mut Buf {
        unsafe { self.0.as_mut() }
    }

    pub fn chan(self) -> usize {
        self.0.as_ptr() as usize
    }

    pub fn cursor_ptr(self) -> *const Buf {
        self.0.as_ptr()
    }

    pub fn into_unsafe_ref(self) -> UnsafeRef<Buf> {
        unsafe { UnsafeRef::from_raw(self.cursor_ptr()) }
    }
}

#[repr(C)]
pub struct Buf {
    pub device_link: LinkedListAtomicLink,
    pub free_link: LinkedListAtomicLink,
    pub io_link: LinkedListAtomicLink,
    pub b_flags: BufFlag,
    pub padding: i32,
    pub b_dev: i16,             // 高8位主设备号，低8位次设备号
    pub b_queue_dev: i16,       // 当前所在设备缓存链，-1 表示未挂到具体设备
    pub b_wcount: i32,          // 需传送的字节数
    pub b_blkno: PhysicalBlock, // 磁盘物理块号
    pub b_error: i32,           // I/O出错信息
    pub b_resid: i32,           // 出错时尚未传送的剩余字节数
    data: Option<NonNull<BufferData>>,
    transfer: Option<NonNull<[u8]>>,
}

impl Buf {
    pub const BLOCK_SIZE: usize = fs_constants::BLOCK_SIZE;

    pub const fn new() -> Self {
        Self {
            device_link: LinkedListAtomicLink::new(),
            free_link: LinkedListAtomicLink::new(),
            io_link: LinkedListAtomicLink::new(),
            b_flags: BufFlag::empty(),
            padding: 0,
            b_dev: -1,
            b_queue_dev: -1,
            b_wcount: 0,
            b_blkno: PhysicalBlock(0),
            b_error: 0,
            b_resid: 0,
            data: None,
            transfer: None,
        }
    }

    pub fn read_table(&self) -> &[i32; 128] {
        unsafe { &*(self.as_slice().as_ptr() as *const [i32; 128]) }
    }

    pub fn write_table(&mut self) -> &mut [i32; 128] {
        unsafe { &mut *(self.as_slice_mut().as_mut_ptr() as *mut [i32; 128]) }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.data_ref().as_slice()
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        self.data_mut().as_slice_mut()
    }

    pub fn io_addr(&self) -> *mut u8 {
        self.transfer
            .map(|transfer| transfer.as_ptr() as *mut u8)
            .unwrap_or_else(|| self.data_ref().as_ptr())
    }

    pub unsafe fn set_transfer(&mut self, addr: *mut u8, len: usize) {
        self.transfer = Some(NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
            addr, len,
        )));
    }

    pub fn clear_transfer(&mut self) {
        self.transfer = None;
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

    pub fn is_on_device_list(&self) -> bool {
        self.device_link.is_linked()
    }

    pub fn is_on_free_list(&self) -> bool {
        self.free_link.is_linked()
    }

    pub fn is_on_io_queue(&self) -> bool {
        self.io_link.is_linked()
    }

    fn data_ref(&self) -> &BufferData {
        unsafe { self.data.expect("buffer data is not attached").as_ref() }
    }

    fn data_mut(&mut self) -> &mut BufferData {
        unsafe { self.data.expect("buffer data is not attached").as_mut() }
    }
}

intrusive_adapter!(pub BufDeviceAdapter = UnsafeRef<Buf>: Buf {
    device_link: LinkedListAtomicLink
});
intrusive_adapter!(pub BufFreeAdapter = UnsafeRef<Buf>: Buf {
    free_link: LinkedListAtomicLink
});
intrusive_adapter!(pub BufIoAdapter = UnsafeRef<Buf>: Buf {
    io_link: LinkedListAtomicLink
});

pub type BufDeviceList = LinkedList<BufDeviceAdapter>;
pub type BufFreeList = LinkedList<BufFreeAdapter>;
pub type BufIoQueue = LinkedList<BufIoAdapter>;

#[repr(C, align(512))]
pub struct BufferData {
    bytes: [u8; Buf::BLOCK_SIZE],
}

impl BufferData {
    pub const fn new() -> Self {
        Self {
            bytes: [0; Buf::BLOCK_SIZE],
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.bytes.as_ptr() as *mut u8
    }
}

#[repr(C)]
pub struct BufferSlot {
    pub buf: Buf,
    pub data: BufferData,
}

impl BufferSlot {
    pub const fn new() -> Self {
        Self {
            buf: Buf::new(),
            data: BufferData::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.buf = Buf::new();
        self.buf.attach_owned_data(&mut self.data);
    }
}

impl Buf {
    fn attach_owned_data(&mut self, data: &mut BufferData) {
        self.data = Some(NonNull::from(data));
        self.transfer = None;
    }
}
