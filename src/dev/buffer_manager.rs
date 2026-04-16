use alloc::boxed::Box;
use eonix_spin::{NoContext, Spin, SpinGuard};
use core::{array, cell::UnsafeCell};

use eonix_sync_base::LazyLock;
use intrusive_collections::{LinkedList, UnsafeRef};

use crate::sync::SpinExt;

use super::{
    block_device::block_device_for_dev,
    buffer::{Buf, BufFlag, BufFreeAdapter, DevId, PhysicalBlock},
    device_manager::ROOTDEV,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    InvalidDevice,
    BufferUnavailable,
    IoError,
    InvalidBuffer,
}

pub type BufferResult<T> = Result<T, BufferError>;

pub struct BufferManager {
    free_list: LinkedList<BufFreeAdapter>,
    free_list_wanted: bool,
    swap_buf: Buf,
    buffers: Box<[Buf; Self::NBUF]>,
    data: Box<[[u8; Self::BUFFER_SIZE]; Self::NBUF]>,
}

unsafe impl Send for BufferManager {}
unsafe impl Sync for BufferManager {}

impl BufferManager {
    pub const NBUF: usize = 15;
    pub const BUFFER_SIZE: usize = Buf::BLOCK_SIZE;

    pub fn new() -> Self {
        let mut manager = Self {
            free_list: LinkedList::new(BufFreeAdapter::NEW),
            free_list_wanted: false,
            swap_buf: Buf::new(),
            buffers: Box::new(array::from_fn(|_| Buf::new())),
            data: Box::new([[0; Self::BUFFER_SIZE]; Self::NBUF]),
        };
        manager.initialize_buffers();
        manager
    }

    fn initialize_buffers(&mut self) {
        self.free_list = LinkedList::new(BufFreeAdapter::NEW);
        self.free_list_wanted = false;
        self.swap_buf = Buf::new();

        for index in 0..Self::NBUF {
            let bp = &mut self.buffers[index] as *mut Buf;

            unsafe {
                *bp = Buf::new();
                (*bp).b_dev = -1;
                (*bp).b_addr = self.data[index].as_mut_ptr();
                self.free_list.push_back(Self::buf_ref(bp));
            }
        }
    }

    pub fn get_blk(&mut self, dev: DevId, blkno: PhysicalBlock) -> BufferResult<*mut Buf> {
        self.validate_device(dev)?;

        if let Some(bp) = self.in_core(dev, blkno) {
            unsafe {
                if (*bp).b_flags.contains(BufFlag::B_BUSY) {
                    (*bp).b_flags.insert(BufFlag::B_WANTED);
                    // TODO: Sleep(bp as usize, PRIBIO), then retry get_blk from the top.
                    return Err(BufferError::BufferUnavailable);
                }
            }

            self.not_avail(bp);
            return Ok(bp);
        }

        let bp = self
            .find_reusable_free_buffer()
            .ok_or(BufferError::BufferUnavailable)?;

        self.not_avail(bp);

        unsafe {
            (*bp).b_flags = BufFlag::B_BUSY;
            self.remove_from_current_device_queue(bp);
            (*bp).b_dev = dev.0;
            (*bp).b_blkno = blkno;
        }

        self.insert_into_device_queue(dev, bp)?;

        Ok(bp)
    }

    pub fn brelse(&mut self, bp: *mut Buf) {
        if bp.is_null() {
            return;
        }

        unsafe {
            if (*bp).b_flags.contains(BufFlag::B_WANTED) {
                // TODO: WakeUpAll(bp as usize).
            }

            if self.free_list_wanted {
                self.free_list_wanted = false;
                // TODO: WakeUpAll(&self.free_list as *const _ as usize).
            }

            (*bp)
                .b_flags
                .remove(BufFlag::B_WANTED | BufFlag::B_BUSY | BufFlag::B_ASYNC);

            if !(*bp).free_link.is_linked() {
                self.free_list.push_back(Self::buf_ref(bp));
            }
        }
    }

    pub fn io_wait(&mut self, bp: *mut Buf) -> BufferResult<()> {
        if bp.is_null() {
            return Err(BufferError::InvalidBuffer);
        }

        unsafe {
            if !(*bp).b_flags.contains(BufFlag::B_DONE) {
                (*bp).b_flags.insert(BufFlag::B_WANTED);
                // TODO: Sleep(bp as usize, PRIBIO), then continue waiting until B_DONE.
                return Err(BufferError::BufferUnavailable);
            }
        }

        self.get_error(bp)
    }

    /// TODO
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
                // TODO: WakeUpAll(bp as usize).
            }
        }
    }

    pub fn bread(&mut self, dev: DevId, blkno: PhysicalBlock) -> BufferResult<*mut Buf> {
        let bp = self.get_blk(dev, blkno)?;

        unsafe {
            if (*bp).b_flags.contains(BufFlag::B_DONE) {
                return Ok(bp);
            }

            (*bp).b_flags.insert(BufFlag::B_READ);
            (*bp).b_wcount = Self::BUFFER_SIZE as i32;
        }

        let device = block_device_for_dev(dev.0).ok_or(BufferError::InvalidDevice)?;
        device.strategy(bp);
        self.io_wait(bp)?;

        Ok(bp)
    }

    pub fn breada(
        &mut self,
        dev: DevId,
        blkno: PhysicalBlock,
        read_ahead_blkno: Option<PhysicalBlock>,
    ) -> BufferResult<*mut Buf> {
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
                match self.get_blk(dev, read_ahead_blkno) {
                    Ok(read_ahead_bp) => unsafe {
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
                    },
                    Err(BufferError::BufferUnavailable) => {
                        // TODO: C++ GetBlk() would sleep here and retry the read-ahead path.
                    }
                    Err(err) => return Err(err),
                }
            }
        }

        match bp {
            Some(bp) => {
                self.io_wait(bp)?;
                Ok(bp)
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
        if self.swap_buf.b_flags.contains(BufFlag::B_BUSY) {
            if self.swap_buf.b_flags.contains(BufFlag::B_DONE) {
                return self.finish_swap();
            }

            self.swap_buf.b_flags.insert(BufFlag::B_WANTED);
            // TODO: Sleep(&self.swap_buf as *const _ as usize, PSWP), then retry swap.
            return Err(BufferError::BufferUnavailable);
        }

        self.swap_buf.b_flags = BufFlag::B_BUSY | flag;
        self.swap_buf.b_dev = ROOTDEV;
        self.swap_buf.b_wcount = count as i32;
        self.swap_buf.b_blkno = blkno;
        self.swap_buf.b_addr = addr as *mut u8;

        let bp = &mut self.swap_buf as *mut Buf;
        let device = block_device_for_dev(ROOTDEV).ok_or(BufferError::InvalidDevice)?;
        device.strategy(bp);

        if !self.swap_buf.b_flags.contains(BufFlag::B_DONE) {
            // TODO: Sleep(&self.swap_buf as *const _ as usize, PSWP), then keep waiting.
            return Err(BufferError::BufferUnavailable);
        }

        self.finish_swap()
    }

    fn finish_swap(&mut self) -> BufferResult<()> {
        let bp = &mut self.swap_buf as *mut Buf;

        if self.swap_buf.b_flags.contains(BufFlag::B_WANTED) {
            // TODO: WakeUpAll(&self.swap_buf as *const _ as usize).
        }

        self.swap_buf
            .b_flags
            .remove(BufFlag::B_BUSY | BufFlag::B_WANTED);

        self.get_error(bp)
    }

    pub fn free_list(&self) -> &LinkedList<BufFreeAdapter> {
        &self.free_list
    }

    pub fn free_list_mut(&mut self) -> &mut LinkedList<BufFreeAdapter> {
        &mut self.free_list
    }

    pub fn swap_buf(&self) -> &Buf {
        &self.swap_buf
    }

    pub fn swap_buf_mut(&mut self) -> &mut Buf {
        &mut self.swap_buf
    }

    pub fn buffers(&self) -> &[Buf; Self::NBUF] {
        self.buffers.as_ref()
    }

    pub fn buffers_mut(&mut self) -> &mut [Buf; Self::NBUF] {
        self.buffers.as_mut()
    }

    pub fn data(&self) -> &[[u8; Self::BUFFER_SIZE]; Self::NBUF] {
        self.data.as_ref()
    }

    pub fn data_mut(&mut self) -> &mut [[u8; Self::BUFFER_SIZE]; Self::NBUF] {
        self.data.as_mut()
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
            let mut cursor = self.free_list.cursor_mut_from_ptr(bp as *const Buf);
            cursor.remove();
            (*bp).b_flags.insert(BufFlag::B_BUSY);
        }
    }

    fn in_core(&self, dev: DevId, blkno: PhysicalBlock) -> Option<*mut Buf> {
        if dev.0 < 0 {
            return None;
        }

        let device = block_device_for_dev(dev.0)?;
        let devtab = device.devtab().lock();
        let mut cursor = devtab.buffers.front();

        while let Some(bp) = cursor.get() {
            if bp.b_dev == dev.0 && bp.b_blkno == blkno {
                return cursor
                    .clone_pointer()
                    .map(|buf| UnsafeRef::into_raw(buf) as *mut Buf);
            }

            cursor.move_next();
        }

        None
    }

    fn validate_device(&self, dev: DevId) -> BufferResult<()> {
        if dev.0 < 0 || block_device_for_dev(dev.0).is_some() {
            Ok(())
        } else {
            Err(BufferError::InvalidDevice)
        }
    }

    fn find_reusable_free_buffer(&mut self) -> Option<*mut Buf> {
        let mut cursor = self.free_list.front();

        while let Some(bp) = cursor.get() {
            if !bp.b_flags.contains(BufFlag::B_DELWRI) {
                return cursor
                    .clone_pointer()
                    .map(|buf| UnsafeRef::into_raw(buf) as *mut Buf);
            }

            cursor.move_next();
        }

        self.free_list_wanted = true;
        // TODO: Sleep(&self.free_list as *const _ as usize, PRIBIO), then retry get_blk.
        None
    }

    fn find_delayed_write_buffer(&self, dev: Option<DevId>) -> Option<*mut Buf> {
        let mut cursor = self.free_list.front();

        while let Some(bp) = cursor.get() {
            let matches_dev = dev.is_none_or(|dev| bp.b_dev == dev.0);

            if matches_dev && bp.b_flags.contains(BufFlag::B_DELWRI) {
                return cursor
                    .clone_pointer()
                    .map(|buf| UnsafeRef::into_raw(buf) as *mut Buf);
            }

            cursor.move_next();
        }

        None
    }

    fn insert_into_device_queue(&self, dev: DevId, bp: *mut Buf) -> BufferResult<()> {
        if dev.0 < 0 {
            return Ok(());
        }

        let device = block_device_for_dev(dev.0).ok_or(BufferError::InvalidDevice)?;
        let mut devtab = device.devtab().lock();

        unsafe {
            devtab.buffers.push_front(Self::buf_ref(bp));
        }

        Ok(())
    }

    unsafe fn remove_from_current_device_queue(&self, bp: *mut Buf) {
        let dev = (*bp).b_dev;
        if dev < 0 {
            return;
        }

        let Some(device) = block_device_for_dev(dev) else {
            return;
        };

        let mut devtab = device.devtab().lock();
        let mut cursor = devtab.buffers.cursor_mut_from_ptr(bp as *const Buf);
        cursor.remove();
    }

    unsafe fn buf_ref(bp: *mut Buf) -> UnsafeRef<Buf> {
        UnsafeRef::from_raw(bp as *const Buf)
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_BUFFER_MANAGER: LazyLock<Spin<BufferManager>> =
    LazyLock::new(|| Spin::new(BufferManager::new()));

pub(crate) fn global_buffer_manager() -> SpinGuard<'static, BufferManager, NoContext> {
    GLOBAL_BUFFER_MANAGER.lock()
}
