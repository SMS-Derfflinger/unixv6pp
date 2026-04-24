use crate::dev::buffer::Buffer;
use crate::proc::ProcessManager;
use crate::sync::{IrqGuard, SuperCell};
use crate::{constants::PosixError, user::Userspace};
use intrusive_collections::UnsafeRef;

use super::{
    block_device::block_device_for_dev,
    buffer::{
        Buf, BufDeviceAdapter, BufDeviceList, BufFlag, BufFreeAdapter, BufFreeList, BufferSlot,
        DevId, PhysicalBlock,
    },
    device_manager::{set_minor, ROOTDEV},
};

pub const PRIBIO: i32 = -50;
pub const PPIPE: u32 = 1;
pub const PSWP: i32 = -100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    InvalidDevice,
    BufferUnavailable,
    IoError,
    InvalidBuffer,
}

impl From<BufferError> for PosixError {
    fn from(value: BufferError) -> Self {
        match value {
            BufferError::InvalidDevice => PosixError::ENXIO,
            BufferError::BufferUnavailable => PosixError::EIO,
            BufferError::IoError => PosixError::EIO,
            BufferError::InvalidBuffer => PosixError::EINVAL,
        }
    }
}

pub type BufferResult<T> = Result<T, BufferError>;

pub struct BufferManager {
    b_free_list: Buf,
    free_list: BufFreeList,
    unassigned_list: BufDeviceList,
    swap_buf: Buf,
    slots: [BufferSlot; Self::NBUF],
}

unsafe impl Send for BufferManager {}
unsafe impl Sync for BufferManager {}

impl BufferManager {
    pub const NBUF: usize = 15;
    pub const BUFFER_SIZE: usize = Buf::BLOCK_SIZE;

    pub const fn new() -> Self {
        Self {
            b_free_list: Buf::new(),
            free_list: BufFreeList::new(BufFreeAdapter::NEW),
            unassigned_list: BufDeviceList::new(BufDeviceAdapter::NEW),
            swap_buf: Buf::new(),
            slots: [const { BufferSlot::new() }; Self::NBUF],
        }
    }

    pub fn initialize(&mut self) {
        self.initialize_buffers();
    }

    fn initialize_buffers(&mut self) {
        self.b_free_list = Buf::new();
        self.free_list = BufFreeList::new(BufFreeAdapter::NEW);
        self.unassigned_list = BufDeviceList::new(BufDeviceAdapter::NEW);
        self.swap_buf = Buf::new();

        for index in 0..Self::NBUF {
            let bp = {
                let slot = &mut self.slots[index];
                slot.initialize();
                &mut slot.buf as *mut Buf
            };

            unsafe {
                (*bp).b_dev = -1;
                self.insert_into_unassigned_queue(bp);
                self.push_free_back(bp);
            }
        }
    }

    pub fn get_blk(&mut self, dev: DevId, blkno: PhysicalBlock) -> BufferResult<*mut Buf> {
        self.validate_device(dev)?;

        loop {
            if let Some(bp) = self.in_core(dev, blkno) {
                unsafe {
                    let ctx = IrqGuard::disable_save();
                    if (*bp).b_flags.contains(BufFlag::B_BUSY) {
                        (*bp).b_flags.insert(BufFlag::B_WANTED);
                        sleep_on(bp as usize, PRIBIO);
                        continue;
                    }
                }

                self.not_avail(bp);
                return Ok(bp);
            }

            let bp = loop {
                unsafe {
                    let ctx = IrqGuard::disable_save();
                    if !self.is_free_list_empty() {
                        let bp = self.free_list_head().expect("free list is not empty");
                        break bp;
                    }

                    self.b_free_list.b_flags.insert(BufFlag::B_WANTED);
                    sleep_on(self.free_list_wait_chan(), PRIBIO);
                }
            };

            self.not_avail(bp);

            unsafe {
                if (*bp).b_flags.contains(BufFlag::B_DELWRI) {
                    (*bp).b_flags.insert(BufFlag::B_ASYNC);
                    self.bwrite(bp)?;
                    continue;
                }

                (*bp).b_flags = BufFlag::B_BUSY;
                self.remove_from_current_device_queue(bp);
                (*bp).b_dev = dev.0;
                (*bp).b_blkno = blkno;
            }

            self.insert_into_device_queue(dev, bp)?;

            return Ok(bp);
        }
    }

    pub fn brelse(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            if (*bp).b_flags.contains(BufFlag::B_WANTED) {
                wakeup_all(bp as usize);
            }

            if self.b_free_list.b_flags.contains(BufFlag::B_WANTED) {
                self.b_free_list.b_flags.remove(BufFlag::B_WANTED);
                wakeup_all(self.free_list_wait_chan());
            }

            if (*bp).b_flags.contains(BufFlag::B_ERROR) {
                (*bp).b_dev = set_minor((*bp).b_dev, -1);
            }

            let ctx = IrqGuard::disable_save();
            (*bp)
                .b_flags
                .remove(BufFlag::B_WANTED | BufFlag::B_BUSY | BufFlag::B_ASYNC);

            if !(*bp).is_on_free_list() {
                self.push_free_back(bp);
            }
        }
    }

    pub fn io_wait(&mut self, bp: *mut Buf) -> BufferResult<()> {
        if bp.is_null() {
            return Err(BufferError::InvalidBuffer);
        }

        unsafe {
            let mut ctx = IrqGuard::disable_save();
            while !(*bp).b_flags.contains(BufFlag::B_DONE) {
                (*bp).b_flags.insert(BufFlag::B_WANTED);
                drop(ctx);
                sleep_on(bp as usize, PRIBIO);
                ctx = IrqGuard::disable_save();
            }
        }

        self.get_error(bp)
    }

    pub fn io_done(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            (*bp).b_flags.insert(BufFlag::B_DONE);
            if (*bp).b_flags.contains(BufFlag::B_ASYNC) {
                self.brelse(bp);
            } else {
                (*bp).b_flags.remove(BufFlag::B_WANTED);
                wakeup_all(bp as usize);
            }
        }
    }

    pub fn bread(&mut self, dev: DevId, blkno: PhysicalBlock) -> BufferResult<Buffer> {
        let bp = self.get_blk(dev, blkno)?;

        unsafe {
            if (*bp).b_flags.contains(BufFlag::B_DONE) {
                return Ok(Buffer::new(bp));
            }

            (*bp).b_flags.insert(BufFlag::B_READ);
            (*bp).b_wcount = Self::BUFFER_SIZE as i32;
        }

        let device = block_device_for_dev(dev.0).ok_or(BufferError::InvalidDevice)?;
        device.strategy(bp);
        self.io_wait(bp)?;

        unsafe { Ok(Buffer::new(bp)) }
    }

    pub fn breada(
        &mut self,
        dev: DevId,
        blkno: PhysicalBlock,
        read_ahead_blkno: Option<PhysicalBlock>,
    ) -> BufferResult<Buffer> {
        self.validate_device(dev)?;

        let mut bp = None;
        let mut should_read_ahead = read_ahead_blkno;

        if self.in_core(dev, blkno).is_none() {
            let new_bp = self.get_blk(dev, blkno)?;

            unsafe {
                if !(*new_bp).b_flags.contains(BufFlag::B_DONE) {
                    (*new_bp).b_flags.insert(BufFlag::B_READ);
                    (*new_bp).b_wcount = Self::BUFFER_SIZE as i32;

                    let device = block_device_for_dev(dev.0).ok_or(BufferError::InvalidDevice)?;
                    device.strategy(new_bp);
                }
            }

            bp = Some(new_bp);
        } else {
            should_read_ahead = None;
        }

        if let Some(read_ahead_blkno) = should_read_ahead {
            if self.in_core(dev, read_ahead_blkno).is_none() {
                let read_ahead_bp = self.get_blk(dev, read_ahead_blkno)?;

                unsafe {
                    if (*read_ahead_bp).b_flags.contains(BufFlag::B_DONE) {
                        self.brelse(read_ahead_bp);
                    } else {
                        (*read_ahead_bp)
                            .b_flags
                            .insert(BufFlag::B_READ | BufFlag::B_ASYNC);
                        (*read_ahead_bp).b_wcount = Self::BUFFER_SIZE as i32;

                        let device =
                            block_device_for_dev(dev.0).ok_or(BufferError::InvalidDevice)?;
                        device.strategy(read_ahead_bp);
                    }
                }
            }
        }

        match bp {
            Some(bp) => {
                self.io_wait(bp)?;
                unsafe { Ok(Buffer::new(bp)) }
            }
            None => self.bread(dev, blkno),
        }
    }

    pub fn bwrite(&mut self, bp: *mut Buf) -> BufferResult<()> {
        if bp.is_null() {
            return Err(BufferError::InvalidBuffer);
        }

        let old_flags = unsafe {
            let bp_ref = &mut *bp;
            let old_flags = bp_ref.b_flags;
            bp_ref
                .b_flags
                .remove(BufFlag::B_READ | BufFlag::B_DONE | BufFlag::B_ERROR | BufFlag::B_DELWRI);
            bp_ref.b_wcount = Self::BUFFER_SIZE as i32;
            old_flags
        };

        let dev = unsafe { (*bp).b_dev };
        let device = block_device_for_dev(dev).ok_or(BufferError::InvalidDevice)?;
        device.strategy(bp);

        if !old_flags.contains(BufFlag::B_ASYNC) {
            self.io_wait(bp)?;
            self.brelse(bp);
        } else if !old_flags.contains(BufFlag::B_DELWRI) {
            self.get_error(bp)?;
        }

        Ok(())
    }

    pub fn bdwrite(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            (*bp).b_flags.insert(BufFlag::B_DELWRI | BufFlag::B_DONE);
        }
        self.brelse(bp);
    }

    pub fn bawrite(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            (*bp).b_flags.insert(BufFlag::B_ASYNC);
        }
        let _ = self.bwrite(bp);
    }

    pub fn clr_buf(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            (*bp).as_slice_mut().fill(0);
        }
    }

    pub fn bflush(&mut self, dev: Option<DevId>) -> BufferResult<()> {
        while let Some(bp) = self.find_delayed_write_buffer(dev) {
            unsafe {
                (*bp).b_flags.insert(BufFlag::B_ASYNC);
            }
            self.not_avail(bp);
            self.bwrite(bp)?;
        }

        Ok(())
    }

    pub fn swap(
        &mut self,
        blkno: PhysicalBlock,
        addr: usize,
        count: usize,
        flag: BufFlag,
    ) -> BufferResult<()> {
        unsafe {
            let ctx = IrqGuard::disable_save();
            while self.swap_buf.b_flags.contains(BufFlag::B_BUSY) {
                self.swap_buf.b_flags.insert(BufFlag::B_WANTED);
                sleep_on(self.swap_buf_ptr() as usize, PSWP);
            }
        }

        self.swap_buf.b_flags = BufFlag::B_BUSY | flag;
        self.swap_buf.b_dev = ROOTDEV;
        self.swap_buf.b_wcount = count as i32;
        self.swap_buf.b_blkno = blkno;
        unsafe {
            self.swap_buf.set_transfer(addr as *mut u8, count);
        }

        let bp = self.swap_buf_ptr();
        let device = block_device_for_dev(ROOTDEV).ok_or(BufferError::InvalidDevice)?;
        device.strategy(bp);

        unsafe {
            let ctx = IrqGuard::disable_save();
            while !self.swap_buf.b_flags.contains(BufFlag::B_DONE) {
                self.swap_buf.b_flags.insert(BufFlag::B_WANTED);
                sleep_on(self.swap_buf_ptr() as usize, PSWP);
            }
        }

        self.finish_swap()
    }

    fn finish_swap(&mut self) -> BufferResult<()> {
        let bp = self.swap_buf_ptr();

        if self.swap_buf.b_flags.contains(BufFlag::B_WANTED) {
            unsafe {
                wakeup_all(bp as usize);
            }
        }

        self.swap_buf
            .b_flags
            .remove(BufFlag::B_BUSY | BufFlag::B_WANTED);
        self.swap_buf.clear_transfer();

        self.get_error(bp)
    }

    pub fn free_list(&self) -> &Buf {
        &self.b_free_list
    }

    pub fn free_list_mut(&mut self) -> &mut Buf {
        &mut self.b_free_list
    }

    pub fn swap_buf(&self) -> &Buf {
        &self.swap_buf
    }

    pub fn swap_buf_mut(&mut self) -> &mut Buf {
        &mut self.swap_buf
    }

    pub fn slots(&self) -> &[BufferSlot; Self::NBUF] {
        &self.slots
    }

    pub fn slots_mut(&mut self) -> &mut [BufferSlot; Self::NBUF] {
        &mut self.slots
    }

    fn get_error(&mut self, bp: *mut Buf) -> BufferResult<()> {
        if bp.is_null() {
            return Err(BufferError::InvalidBuffer);
        }

        unsafe {
            if (*bp).b_flags.contains(BufFlag::B_ERROR) {
                Err(BufferError::IoError)
            } else {
                Ok(())
            }
        }
    }

    fn not_avail(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            let ctx = IrqGuard::disable_save();
            if (*bp).is_on_free_list() {
                self.remove_from_free_list(bp);
            }
            (*bp).b_flags.insert(BufFlag::B_BUSY);
        }
    }

    fn in_core(&self, dev: DevId, blkno: PhysicalBlock) -> Option<*mut Buf> {
        if dev.0 < 0 {
            return self.in_device_list(&self.unassigned_list, blkno, dev.0);
        }

        let device = block_device_for_dev(dev.0)?;
        device
            .devtab()
            .with(|devtab| self.in_device_list(&devtab.buffer_list, blkno, dev.0))
    }

    fn validate_device(&self, dev: DevId) -> BufferResult<()> {
        if dev.0 < 0 || block_device_for_dev(dev.0).is_some() {
            Ok(())
        } else {
            Err(BufferError::InvalidDevice)
        }
    }

    fn find_delayed_write_buffer(&self, dev: Option<DevId>) -> Option<*mut Buf> {
        for bp in self.free_list.iter() {
            let matches_dev = dev.is_none_or(|dev| bp.b_dev == dev.0);

            if matches_dev && bp.b_flags.contains(BufFlag::B_DELWRI) {
                return Some(bp as *const Buf as *mut Buf);
            }
        }

        None
    }

    fn insert_into_device_queue(&mut self, dev: DevId, bp: *mut Buf) -> BufferResult<()> {
        if dev.0 < 0 {
            self.insert_into_unassigned_queue(bp);
            return Ok(());
        }

        let device = block_device_for_dev(dev.0).ok_or(BufferError::InvalidDevice)?;
        device.devtab().with_mut(|devtab| unsafe {
            Self::insert_device_front(&mut devtab.buffer_list, bp, dev.0);
        });

        Ok(())
    }

    unsafe fn remove_from_current_device_queue(&mut self, bp: *mut Buf) {
        if !(*bp).is_on_device_list() {
            return;
        }

        let queue_dev = (*bp).b_queue_dev;
        if queue_dev < 0 {
            Self::remove_from_device_list(&mut self.unassigned_list, bp);
            return;
        }

        if let Some(device) = block_device_for_dev(queue_dev) {
            device
                .devtab()
                .with_mut(|devtab| Self::remove_from_device_list(&mut devtab.buffer_list, bp));
        }
    }

    fn in_device_list(
        &self,
        list: &BufDeviceList,
        blkno: PhysicalBlock,
        dev: i16,
    ) -> Option<*mut Buf> {
        for bp in list.iter() {
            if bp.b_blkno == blkno && bp.b_dev == dev {
                return Some(bp as *const Buf as *mut Buf);
            }
        }

        None
    }

    fn insert_into_unassigned_queue(&mut self, bp: *mut Buf) {
        unsafe {
            Self::insert_device_front(&mut self.unassigned_list, bp, -1);
        }
    }

    unsafe fn insert_device_front(list: &mut BufDeviceList, bp: *mut Buf, queue_dev: i16) {
        if (*bp).is_on_device_list() {
            return;
        }

        (*bp).b_queue_dev = queue_dev;
        list.push_front(UnsafeRef::from_raw(bp as *const Buf));
    }

    fn remove_from_device_list(list: &mut BufDeviceList, bp: *mut Buf) {
        let mut cursor = unsafe { list.cursor_mut_from_ptr(bp as *const Buf) };
        cursor.remove();

        unsafe {
            (*bp).b_queue_dev = -1;
        }
    }

    unsafe fn push_free_back(&mut self, bp: *mut Buf) {
        if (*bp).is_on_free_list() {
            return;
        }

        self.free_list
            .push_back(UnsafeRef::from_raw(bp as *const Buf));
    }

    unsafe fn remove_from_free_list(&mut self, bp: *mut Buf) {
        let mut cursor = self.free_list.cursor_mut_from_ptr(bp as *const Buf);
        cursor.remove();
    }

    fn is_free_list_empty(&self) -> bool {
        self.free_list.is_empty()
    }

    fn free_list_head(&self) -> Option<*mut Buf> {
        self.free_list
            .front()
            .clone_pointer()
            .map(|bp| UnsafeRef::into_raw(bp) as *mut Buf)
    }

    fn free_list_wait_chan(&mut self) -> usize {
        &mut self.b_free_list as *mut Buf as usize
    }

    fn swap_buf_ptr(&mut self) -> *mut Buf {
        &mut self.swap_buf as *mut Buf
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_BUFFER_MANAGER: SuperCell<BufferManager> = SuperCell::new(BufferManager::new());

pub(crate) fn global_buffer_manager() -> &'static mut BufferManager {
    GLOBAL_BUFFER_MANAGER.get_mut()
}

unsafe fn sleep_on(chan: usize, pri: i32) {
    Userspace::get().proc().sleep_kernel(chan, pri);
}

unsafe fn wakeup_all(chan: usize) {
    ProcessManager::get().wakeup_all(chan);
}

pub fn buffer_manager_initialize() {
    global_buffer_manager().initialize();
}
