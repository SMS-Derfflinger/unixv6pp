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
use kernel_macros::define_class_compat;

use crate::{
    compat::{compat_phys_copy, compat_swap_alloc, compat_swap_free},
    constants::Signal,
    dev::buffer::PhysicalBlock,
    fs::{InodeRef, OpenFiles, GLOBAL_OPEN_FILE_TABLE},
    mm::{PhysPage, KERNEL_PAGE_MANAGER, PAGE_SIZE, USER_PAGE_MANAGER},
    proc::{
        context::TaskContext,
        manager::{ProcessManager, SLOAD, SSWAP},
        Channel,
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
pub struct TrapFrame {
    pub eip: usize,
    pub xcs: usize,
    pub eflags: usize,
    pub esp: *mut usize,
    pub xss: usize,
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
        inode.lock().i_count += 1;

        let aligned_size = len.next_power_of_two();
        let order = aligned_size.trailing_zeros() - 12;

        let text = Box::new(Text {
            disk_addr: compat_swap_alloc(len),
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

        compat_swap_free(self.disk_addr.0);

        unsafe {
            let _ = Box::from_raw(&raw mut *self);
        }
    }
}

pub struct Terminal;

pub struct KernelStack {
    pages: Option<&'static mut PhysPage>,
}

/// 内核栈大小：order=3 即 2^3 = 8 页 = 32KB
const KSTACK_ORDER: u32 = 3;
const KSTACK_SIZE: usize = (1 << KSTACK_ORDER as usize) * PAGE_SIZE;

impl KernelStack {
    pub fn new() -> Self {
        Self {
            pages: Some(
                KERNEL_PAGE_MANAGER
                    .lock()
                    .alloc_order(KSTACK_ORDER)
                    .expect("Out of kernel memory for stack"),
            ),
        }
    }

    /// 返回内核栈顶的虚拟地址（栈从高地址向低地址增长）
    pub fn top(&self) -> usize {
        let phys = self.pages.as_ref().unwrap().phys();
        crate::mm::phys_to_virt(phys) as usize + KSTACK_SIZE
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        if let Some(pages) = self.pages.take() {
            unsafe {
                KERNEL_PAGE_MANAGER.lock().dealloc(pages);
            }
        }
    }
}

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
    pub ctx: TaskContext,

    /// 每个进程独立的内核栈
    pub kstack: Option<KernelStack>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

pub(crate) trait KResultExt {
    fn pass_to_user(self);
}

pub(crate) trait NativeWord {
    fn into_word(self) -> usize;
}

impl NativeWord for u32 {
    fn into_word(self) -> usize {
        self as usize
    }
}

impl NativeWord for usize {
    fn into_word(self) -> usize {
        self
    }
}

impl NativeWord for () {
    fn into_word(self) -> usize {
        0
    }
}

impl<T: NativeWord> KResultExt for KResult<T> {
    fn pass_to_user(self) {
        match self {
            Ok(retval) => Userspace::get().set_user_retval(retval.into_word() as u32),
            Err(err) => Userspace::get().set_error(err),
        }
    }
}

impl Process {
    pub fn setuid(&mut self, uid: u16) {
        self.uid = uid;
    }

    pub fn send_signal(&mut self, signal: u32, func: usize) -> KResult<usize> {
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

    pub fn process_signal(&mut self, context: &mut TrapFrame) {
        let Some(pending) = self.pending_signal.take() else {
            crate::println_warn!("Signal UNKNOWN");
            self.exit();
        };

        Userspace::get().clear_error();
        let old_eip = context.eip;
        context.eip = Userspace::get().get_signal_handler(pending);
        context.esp = context.esp.wrapping_sub(1);

        unsafe {
            context.esp.write(old_eip);
        }
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
        crate::println_info!("{signal:?} triggered");

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

    pub fn sleep_kernel(&mut self, chan: impl Channel, pri: i32) {
        {
            let ctx = IrqGuard::disable_save();
            self.wchan = chan.channel_addr();
            self.stat = ProcessState::SSLEEP;
            self.pri = pri;
        }

        ProcessManager::get().switch();
    }

    /// Interruptible sleep.
    ///
    /// # Returns
    /// Whether we have pending signals.
    pub fn sleep_user(&mut self, chan: usize, pri: u32) -> bool {
        if self.should_process() {
            return true;
        }

        {
            let ctx = IrqGuard::disable_save();
            self.wchan = chan;
            self.stat = ProcessState::SWAIT;
            self.pri = pri as i32;
        }

        if ProcessManager::get().run_in != 0 {
            ProcessManager::get().run_in = 0;
            ProcessManager::get().wakeup_runin();
        }

        ProcessManager::get().switch();

        self.should_process()
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
        compat_phys_copy(PAddr::from_val(old_addr), new_addr, copylen);

        if let Some(pages) = self.pages.replace(new_page) {
            unsafe {
                USER_PAGE_MANAGER.lock().dealloc(pages);
            }
        }

        Userspace::get().mem.map_to_actual_pt(self);
    }

    pub fn sstack(&mut self) -> KResult<()> {
        let change = PAGE_SIZE;
        let mem = &mut Userspace::get().mem;
        mem.stack_len += change;

        let newlen = PAGE_SIZE + mem.data_len + mem.stack_len;
        mem.establish_user(self)?;

        self.expand(newlen);
        let mut dst = PAddr::from_val(self.addr + newlen);
        let mut cnt = mem.stack_len - change;

        while cnt != 0 {
            cnt -= 1;
            dst = dst - 1;
            compat_phys_copy(dst - change, dst, 1);
        }

        mem.map_to_actual_pt(self);
        Ok(())
    }

    pub fn sbrk(&mut self, brk: usize) -> KResult<usize> {
        let mem = &mut Userspace::get().mem;
        let newlen = brk - mem.data;

        if brk == 0 {
            return Ok(mem.data + mem.data_len);
        }

        mem.establish_user(self)?;

        if newlen == mem.data_len {
            return Ok(brk);
        }

        let change = newlen as isize - mem.data_len as isize;
        mem.data_len = newlen;
        let newlen = newlen + PAGE_SIZE + mem.stack_len;

        if change < 0 {
            let change_abs = -change as usize;
            let mut dst = PAddr::from_val(self.addr + newlen - mem.stack_len);
            let mut cnt = mem.stack_len;

            while cnt != 0 {
                cnt -= 1;
                compat_phys_copy(dst + change_abs, dst, 1);
                dst = dst + 1;
            }
            self.expand(newlen);
        } else {
            self.expand(newlen);
            let mut dst = PAddr::from_val(self.addr + newlen);
            let mut cnt = mem.stack_len;

            while cnt != 0 {
                cnt -= 1;
                dst = dst - 1;
                compat_phys_copy(dst - change as usize, dst, 1);
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
            let Ok(file) = Userspace::get().open_files.get_f(fd) else {
                continue;
            };
            GLOBAL_OPEN_FILE_TABLE.lock().close_f(&file);
            Userspace::get().open_files.clear_f(fd);
        }

        if let Some(_cwd) = Userspace::get().cwd.take() {
            // TODO: put cwd
        }

        let _ = self.text.take();

        // TODO: save exit status
        unsafe {
            USER_PAGE_MANAGER.lock().dealloc(self.pages.take().unwrap());
        }

        self.stat = ProcessState::SZOMB;

        ProcessManager::get().wake_ppid(self.ppid);
        ProcessManager::get().reparent(self.pid);

        ProcessManager::get().switch();

        panic!("This function should never return");
    }
}

define_class_compat! {impl Process {
    pub fn send_signal(&mut self) {
        let signal = Userspace::get().args[0] as u32;
        let func = Userspace::get().args[1];

        this.send_signal(signal, func).pass_to_user();
    }

    pub fn process_signal(&mut self, context: &mut TrapFrame) {
        this.process_signal(context);
    }

    pub fn should_process(&mut self) -> bool {
        this.should_process()
    }

    pub fn raise(&mut self, signal: u32) {
        this.raise_raw(signal).pass_to_user();
    }

    pub fn set_nice(&mut self) {
        let nice = Userspace::get().args[0] as i32;
        this.set_nice(nice);
    }

    pub fn set_pri(&mut self) {
        this.set_pri();
    }

    pub fn exit(&mut self) {
        this.exit();
    }

    pub fn sstack(&mut self) {
        this.sstack().pass_to_user();
    }

    pub fn sbrk(&mut self) {
        let brk = Userspace::get().args[0];
        this.sbrk(brk).pass_to_user();
    }

    pub fn sleep_kernel(&mut self, chan: usize, pri: i32) {
        this.sleep_kernel(chan, pri);
    }
}}

define_class_compat! {impl KernelStack {
    pub fn new() -> *mut u8 {
        let stack = KernelStack::new();
        let top = stack.top() as *mut u8;
        core::mem::forget(stack);
        top
    }
}}
