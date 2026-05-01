use core::{
    num::NonZero,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use alloc::boxed::Box;
use eonix_mm::{
    address::{Addr, PAddr},
    paging::{Folio, PFN},
};

use crate::{
    constants::{PosixError, Signal},
    dev::buffer::PhysicalBlock,
    fs::{InodeRef, OpenFiles},
    interrupt::context::TrapContext,
    machine::switch_user_struct,
    mm::{phys_copy, swap_alloc, swap_free, KernelStack, PhysPage, PAGE_SIZE, USER_PAGE_MANAGER},
    proc::{
        context::TaskContext,
        manager::{SLOAD, SSWAP},
        Channel, ProcessManager,
    },
    serial::KResult,
    sync::{IrqGuard, SpinExt},
    user::Userspace,
};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    SNULL = 0,
    SSLEEP = 1,
    SWAIT = 2,
    SRUN = 3,
    SIDL = 4,
    SZOMB = 5,
    SSTOP = 6,
}

#[repr(C)]
pub struct Text {
    pub disk_addr: PhysicalBlock,
    pub len_bytes: usize,
    pages: Option<&'static mut PhysPage>,
    inode: InodeRef,
    refcount: usize,
    in_mem_count: usize,
}

pub struct TextRef(NonNull<Text>);

unsafe impl Send for TextRef {}
unsafe impl Sync for TextRef {}

impl DerefMut for TextRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

impl Deref for TextRef {
    type Target = Text;
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl Drop for TextRef {
    fn drop(&mut self) {
        self.put();
    }
}

impl TextRef {
    pub fn clone(&mut self) -> Self {
        self.get();
        Self(self.0)
    }
}

impl Text {
    pub fn new(inode: InodeRef, len: usize) -> TextRef {
        let aligned_size = len.next_power_of_two();
        let order = aligned_size.trailing_zeros() - 12;

        let text = Box::new(Text {
            disk_addr: swap_alloc(len),
            pages: USER_PAGE_MANAGER.lock().alloc_order(order),
            len_bytes: len,
            inode,
            refcount: 1,
            in_mem_count: 1,
        });

        let text_ref = TextRef(NonNull::from_ref(&text));
        let _ = Box::into_raw(text);

        text_ref
    }

    pub fn addr(&self) -> Option<usize> {
        self.pfn().map(|pfn| PAddr::from(pfn).addr())
    }

    pub fn pfn(&self) -> Option<PFN> {
        self.pages.as_ref().map(|pages| pages.pfn())
    }

    fn get(&mut self) {
        self.refcount += 1;
        self.in_mem_count += 1;
    }

    pub fn put_mem(&mut self) {
        assert_ne!(self.in_mem_count, 0);
        self.in_mem_count -= 1;

        if self.in_mem_count != 0 {
            return;
        }

        let pages = self.pages.take().unwrap();
        unsafe {
            USER_PAGE_MANAGER.lock().dealloc(pages);
        }
    }

    fn put(&mut self) {
        if self.in_mem_count != 0 {
            self.put_mem();
        }

        assert_ne!(self.refcount, 0);
        self.refcount -= 1;

        if self.refcount != 0 {
            return;
        }

        swap_free(self.disk_addr, self.len_bytes);

        unsafe {
            let _ = Box::from_raw(&raw mut *self);
        }
    }
}

pub struct Terminal;

#[repr(C)]
pub struct Process {
    pub uid: u16,
    pub pid: u32,
    pub ppid: u32,

    pub addr: usize,
    pub size: u32,
    pub text: Option<TextRef>,
    pub stat: ProcessState,
    pub flag: u32,

    pub pri: i32,
    pub cpu: u32,
    pub nice: i32,
    pub time: u32,

    pub wchan: usize,

    pub pending_signal: Option<Signal>,
    pub tty: *const Terminal,
    pub sigmap: usize,
    pub pages: Option<&'static mut PhysPage>,
    pub trap_context: TrapContext,
    pub ctx: TaskContext,

    /// 每个进程独立的内核栈
    pub kstack: KernelStack,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

impl Process {
    pub fn setuid(&mut self, uid: u16) {
        self.uid = uid;
    }

    pub fn init_kernel_context(&mut self, entry: Option<usize>) {
        self.trap_context = TrapContext::new();
        self.trap_context.kernel_sp = self.kstack.top().addr().get();

        self.ctx.set_stack_pointer(self.kstack.top().addr().get());
        if let Some(entry) = entry {
            self.ctx.set_program_counter(entry);
        }
    }

    pub fn trap_context_ptr(&mut self) -> *mut TrapContext {
        &raw mut self.trap_context
    }

    pub fn task_context_ptr(&mut self) -> *mut TaskContext {
        &raw mut self.ctx
    }

    pub fn set_signal_handler(&mut self, signal: u32, func: usize) -> KResult<usize> {
        let signal = Signal::try_from(signal)?;
        let old_handler = Userspace::get().get_signal_handler(signal);
        Userspace::get().set_signal_handler(signal, func);

        if let Some(pending) = self.pending_signal {
            if pending == signal {
                self.pending_signal = None;
            }
        }

        Ok(old_handler)
    }

    pub fn process_signal(&mut self, context: &mut TrapContext) {
        let Some(pending) = self.pending_signal.take() else {
            crate::println_warn!("Signal UNKNOWN");
            self.exit();
        };

        let handler = Userspace::get().get_signal_handler(pending);

        if handler == 0 {
            crate::println_warn!("[{:2}] Killed", self.pid);
            self.exit();
        }

        unsafe {
            (context.regs.sp as *mut u8)
                .wrapping_byte_sub(size_of::<TrapContext>())
                .cast::<TrapContext>()
                .write(*context);
        }

        context.sepc = handler;
        context.regs.ra = 0x10; // sigret_trampoline in vdso
        context.regs.sp -= size_of::<TrapContext>();

        Userspace::get().clear_error();
    }

    pub fn should_process(&self) -> bool {
        if let Some(pending) = self.pending_signal {
            if pending == Signal::SIGINT {
                return true;
            }

            if Userspace::get().get_signal_handler(pending) != 0 {
                return true;
            }
        }

        false
    }

    const PUSER: u32 = 100;

    pub fn set_run(&mut self) {
        let pm = ProcessManager::get();

        self.wchan = 0;
        self.stat = ProcessState::SRUN;

        if self.pri < pm.cur_pri {
            pm.runrun += 1;
        }

        if pm.run_out != 0 && (self.flag & SLOAD) == 0 {
            pm.run_out = 0;
            pm.wakeup_runout();
        }
    }

    pub fn set_pri(&mut self) {
        let mut pri = self.cpu / 16 + Self::PUSER + self.nice as u32;

        if pri > 255 {
            pri = 255;
        }

        if pri as i32 > ProcessManager::get().cur_pri {
            ProcessManager::get().runrun += 1;
        }

        self.pri = pri as i32;
    }

    pub fn raise(&mut self, signal: Signal) {
        crate::println_info!("[{:2}] {signal:?} raised", self.pid);

        // ???
        if signal == Signal::SIGKILL {
            return;
        }

        self.pending_signal = Some(signal);

        if self.pri > Self::PUSER as i32 {
            self.pri = Self::PUSER as i32;
        }

        if self.stat == ProcessState::SWAIT {
            self.set_run();
        }
    }

    fn raise_raw(&mut self, signal: u32) -> KResult<()> {
        let signal = Signal::try_from(signal)?;
        self.raise(signal);
        Ok(())
    }

    pub fn set_nice(&mut self, mut nice: i32) {
        if nice > 20 {
            nice = 20;
        }

        if nice < 0 && self.uid != 0 {
            nice = 0;
        }

        self.nice = nice;
    }

    pub fn is_sleeping_on(&self, chan: usize) -> bool {
        match self.stat {
            ProcessState::SWAIT | ProcessState::SSLEEP => {}
            _ => return false,
        }

        self.wchan == chan
    }

    pub fn set_kernel_sleep(&mut self, chan: impl Channel, pri: i32) {
        let _irq = IrqGuard::disable_save();
        self.wchan = chan.channel_addr();
        self.stat = ProcessState::SSLEEP;
        self.pri = pri;
    }

    pub fn finish_kernel_sleep(&mut self) {
        let _irq = IrqGuard::disable_save();
        self.wchan = 0;
        self.stat = ProcessState::SRUN;
    }

    pub fn sleep_kernel(&mut self, chan: impl Channel, pri: i32) {
        #[cfg(feature = "debug_irq")]
        crate::println_debug!(
            "pid{} sleep kernel chan={:#x}",
            self.pid,
            chan.channel_addr()
        );

        self.set_kernel_sleep(chan, pri);

        ProcessManager::get().switch();
    }

    pub fn sleep_kernel_with_irq_guard(&mut self, chan: impl Channel, pri: i32, ctx: IrqGuard) {
        #[cfg(feature = "debug_irq")]
        crate::println_debug!(
            "pid{} sleep kernel with guard chan={:#x}",
            self.pid,
            chan.channel_addr()
        );
        self.wchan = chan.channel_addr();
        self.stat = ProcessState::SSLEEP;
        self.pri = pri;
        drop(ctx);

        ProcessManager::get().switch();

        #[cfg(feature = "debug_irq")]
        crate::println_debug!(
            "pid{} DONE sleep kernel with guard chan={:#x}",
            self.pid,
            chan.channel_addr()
        );
    }

    /// Interruptible sleep.
    ///
    /// # Returns
    /// Whether we have pending signals.
    #[must_use]
    pub fn sleep_user(&mut self, chan: usize, pri: u32) -> KResult<()> {
        self._sleep_user_with_guard(chan, pri, None)
    }

    #[must_use]
    pub fn sleep_user_with_irq_guard(
        &mut self,
        chan: usize,
        pri: u32,
        irq: IrqGuard,
    ) -> KResult<()> {
        self._sleep_user_with_guard(chan, pri, Some(irq))
    }

    fn _sleep_user_with_guard(
        &mut self,
        chan: usize,
        pri: u32,
        irq: Option<IrqGuard>,
    ) -> KResult<()> {
        #[cfg(feature = "debug_irq")]
        crate::println_debug!("pid{} sleep user chan={chan:#x}", self.pid);

        if self.should_process() {
            return Err(PosixError::EINTR);
        }

        {
            let _ctx = IrqGuard::disable_save();
            self.wchan = chan;
            self.stat = ProcessState::SWAIT;
            self.pri = pri as i32;
        }

        if ProcessManager::get().run_in != 0 {
            ProcessManager::get().run_in = 0;
            ProcessManager::get().wakeup_runin();
        }

        drop(irq);
        ProcessManager::get().switch();

        if self.should_process() {
            return Err(PosixError::EINTR);
        }

        Ok(())
    }

    pub fn expand(&mut self, newlen: usize) {
        let oldlen = self.size as usize;
        self.size = newlen as u32;

        let old_addr = self.addr;

        let aligned_size = newlen.next_power_of_two();
        let order = aligned_size.trailing_zeros() - 12;

        let Some(new_page) = USER_PAGE_MANAGER.lock().alloc_order(order) else {
            ProcessManager::get().send_to_swap(self, true, NonZero::new(oldlen));

            self.flag |= SSWAP;
            ProcessManager::get().switch();
            return;
        };

        let new_addr = new_page.phys();
        self.addr = new_addr.addr();

        let copylen = oldlen.min(newlen);
        phys_copy(PAddr::from_val(old_addr), new_addr, copylen);

        if let Some(pages) = self.pages.replace(new_page) {
            unsafe {
                USER_PAGE_MANAGER.lock().dealloc(pages);
            }
        }

        {
            let _irq = IrqGuard::disable_save();
            switch_user_struct(self);
            Userspace::get().mem.map_to_actual_pt(self);
        }
    }

    pub fn sstack(&mut self) -> KResult<()> {
        let change = PAGE_SIZE;
        let mem = &mut Userspace::get().mem;
        mem.stack_len += change;

        let newlen = mem.resident_len();
        mem.establish_user(self)?;

        self.expand(newlen);
        let mut dst = PAddr::from_val(self.addr + newlen);
        let mut cnt = mem.stack_len - change;

        while cnt != 0 {
            cnt -= 1;
            dst = dst - 1;
            phys_copy(dst - change, dst, 1);
        }

        let _irq = IrqGuard::disable_save();
        mem.map_to_actual_pt(self);
        Ok(())
    }

    pub fn sbrk(&mut self, brk: usize) -> KResult<usize> {
        let mem = &mut Userspace::get().mem;

        if brk == 0 {
            return Ok(mem.data + mem.data_len);
        }

        if brk < mem.data {
            return Err(PosixError::ENOMEM);
        }

        let old_data_len = mem.data_len;
        let old_resident_len = mem.resident_len();
        let new_data_len = brk - mem.data;

        if new_data_len == old_data_len {
            return Ok(brk);
        }

        mem.data_len = new_data_len;
        if let Err(err) = mem.establish_user(self) {
            mem.data_len = old_data_len;
            mem.establish_user(self)?;
            return Err(err);
        }

        let new_resident_len = mem.resident_len();
        let resident_change = new_resident_len as isize - old_resident_len as isize;

        if resident_change < 0 {
            let change_abs = (-resident_change) as usize;
            let mut dst = PAddr::from_val(self.addr + new_resident_len - mem.stack_len);
            let mut cnt = mem.stack_len;

            while cnt != 0 {
                cnt -= 1;
                phys_copy(dst + change_abs, dst, 1);
                dst = dst + 1;
            }
            self.expand(new_resident_len);
        } else if resident_change > 0 {
            self.expand(new_resident_len);
            let mut dst = PAddr::from_val(self.addr + new_resident_len);
            let mut cnt = mem.stack_len;

            while cnt != 0 {
                cnt -= 1;
                dst = dst - 1;
                phys_copy(dst - resident_change as usize, dst, 1);
            }
        }

        Ok(brk)
    }

    pub fn exit(&mut self) -> ! {
        assert_ne!(self.pid, 0, "Trying to kill process #0...");

        crate::println_info!("Process {} is exiting", self.pid);
        // TODO: reset trace flag

        // Ignore all signals

        Userspace::get().clear_signal_handlers();
        for fd in 0..OpenFiles::NOFILES {
            #[cfg(feature = "debug_proc")]
            crate::println_debug!("clear fd{fd}");
            Userspace::get().open_files.clear_f(fd);
        }

        if let Some(_cwd) = Userspace::get().cwd.take() {
            // TODO: put cwd
        }

        // `Userspace` lives inside the process u-area, so Rust never runs its
        // destructor. Free the heap-backed user page-table array explicitly.
        let user_pts = unsafe { core::ptr::read(&Userspace::get().mem.user_pts) };
        drop(user_pts);

        let _ = self.text.take();

        // TODO: save exit status
        if let Some(pages) = self.pages.take() {
            unsafe {
                USER_PAGE_MANAGER.lock().dealloc(pages);
            }
        }

        self.stat = ProcessState::SZOMB;

        ProcessManager::get().wake_ppid(self.ppid);
        ProcessManager::get().reparent(self.pid);

        #[cfg(feature = "debug_proc")]
        crate::println_info!("[{pid:2}] Process {pid} exited", pid = self.pid);

        ProcessManager::get().switch();

        panic!("This function should never return");
    }
}
