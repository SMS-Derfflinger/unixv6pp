use core::{
    arch::naked_asm,
    ffi::CStr,
    num::NonZero,
    ptr::NonNull,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, ffi::CString, vec, vec::Vec};
use eonix_mm::address::{Addr, PAddr};
use eonix_sync_base::LazyLock;
use kernel_macros::define_class_compat;

use crate::{
    compat::{compat_phys_copy, compat_swap_alloc},
    constants::{PosixError, Signal},
    dev::{buffer::BufFlag, buffer_manager::global_buffer_manager},
    fs::{DirSearchMode, FileManager, InodeMode, InodeRefExt},
    interrupt::Registers,
    loader::PEParser,
    machine::{asm::disable_interrupts, set_tss_esp0, switch_user_struct},
    mm::{PAGE_SIZE, USER_PAGE_MANAGER},
    proc::{
        context::TaskContext,
        process::{KResultExt, KernelStack, ProcessState, Terminal, Text, TrapFrame},
        Channel, Process, EXPRI,
    },
    serial::KResult,
    sync::{IrqGuard, SpinExt, SuperCell},
    user::{MemoryDescriptor, Userspace, Userspace_init},
};

static NEXT_PID: AtomicU32 = AtomicU32::new(0);

pub static GLOBAL_PROC_MANAGER: LazyLock<SuperCell<ProcessManager>> =
    LazyLock::new(|| SuperCell::new(ProcessManager::new()));

pub struct ProcessImage();

pub struct ProcessManager {
    procs: Vec<Box<Process>>,

    pub cur_pri: i32,
    pub runrun: u32,
    pub run_in: u32,
    pub run_out: u32,
    exe_cnt: u32,
    pub switch_cnt: u32,
}

pub const SLOAD: u32 = 1 << 0;
pub const SSYS: u32 = 1 << 1;
pub const SLOCK: u32 = 1 << 2;
pub const SSWAP: u32 = 1 << 3;

impl Process {
    pub fn new_from(pid: u32, parent: &mut Self) -> Box<Self> {
        let kstack = KernelStack::new();
        let stack_top = kstack.top();

        let mut child = Box::new(Self {
            uid: parent.uid,
            pid,
            ppid: parent.pid,
            addr: 0,
            size: parent.size,
            text: parent.text.as_mut().map(|text| text.clone()),
            stat: ProcessState::SRUN,
            flag: SLOAD,
            pri: 0,
            cpu: 0,
            nice: parent.nice,
            time: 0,
            wchan: 0,
            pending_signal: None,
            tty: parent.tty,
            sigmap: 0,
            pages: None,
            ctx: TaskContext::new(),
            kstack: Some(kstack),
        });

        // 设置子进程的内核栈指针，使其在被调度时能正确使用自己的栈
        child.ctx.esp = stack_top;

        child
    }
}

impl ProcessManager {
    pub const NTEXT: usize = 64;

    fn assign_pid() -> u32 {
        NEXT_PID.fetch_add(1, Ordering::Acquire)
    }

    pub fn new() -> Self {
        Self {
            procs: vec![],
            cur_pri: 0,
            runrun: 0,
            run_in: 0,
            run_out: 0,
            exe_cnt: 0,
            switch_cnt: 0,
        }
    }

    fn set_init_context(child: &mut Process) {
        let stack_top = child.kstack.as_ref().unwrap().top();

        child.ctx.eip = go_init as usize;
        child.ctx.esp = stack_top;
        child.ctx.ebp = stack_top;
        child.ctx.ebx = 0;
        child.ctx.esi = 0;
        child.ctx.edi = 0;
    }

    fn new_proc(&mut self, parent: &mut Process) -> Box<Process> {
        let mut child = Process::new_from(Self::assign_pid(), parent);
        let mut cur_addr = PAddr::from_val(parent.addr);
        let cur_size = parent.size;

        let mut new_user = Box::new(Userspace::get().clone());
        new_user.proc = &raw mut *child;

        let aligned_size = cur_size.next_power_of_two();
        let order = aligned_size.trailing_zeros() - 12;
        let new_pages = USER_PAGE_MANAGER.lock().alloc_order(order);

        let ctx = IrqGuard::disable_save();
        Userspace::replace(&mut new_user);

        if let Some(pages) = new_pages {
            let mut cnt = parent.size;
            let mut to_addr = pages.phys();
            child.addr = pages.phys().addr();
            while cnt != 0 {
                cnt -= 1;
                compat_phys_copy(cur_addr, to_addr, 1);
                cur_addr = cur_addr + 1;
                to_addr = to_addr + 1;
            }
            // TODO: pages is leaked here...
            //       THIS IS A BUG BUT FIXINT IT WOULD CAUSE THE KERNEL TO CRASH...
        } else {
            parent.stat = ProcessState::SIDL;
            child.addr = parent.addr;
            self.send_to_swap(&mut child, false, None);
            child.flag |= SSWAP;
            parent.stat = ProcessState::SRUN;
        }

        Userspace::replace(&mut new_user);

        core::mem::forget(new_user);
        child
    }

    pub fn raise(&mut self, tty: *const Terminal, signal: Signal) {
        for proc in &mut self.procs {
            if proc.tty != tty {
                continue;
            }

            proc.raise(signal);
        }
    }

    pub fn compat_raise_non_scheduler(&mut self, signal: Signal) {
        for proc in &mut self.procs {
            if proc.pid > 1 {
                continue;
            }

            proc.raise(signal);
        }
    }

    pub fn send_to_swap(
        &mut self,
        proc: &mut Process,
        do_free: bool,
        swap_len: Option<NonZero<usize>>,
    ) {
        let swap_len = swap_len.map(|l| l.get()).unwrap_or(proc.size as usize);

        let blkno = compat_swap_alloc(proc.size as usize);

        if let Some(text) = &mut proc.text {
            text.put_mem();
        }

        proc.flag |= SLOCK;
        global_buffer_manager()
            .swap(blkno, proc.addr, swap_len, BufFlag::B_WRITE)
            .expect("Swap I/O Error");

        if do_free {
            let pages = proc.pages.take().unwrap();
            unsafe {
                USER_PAGE_MANAGER.lock().dealloc(pages);
            }
        }

        // (flag & SLOAD) => addr == blkno
        proc.addr = blkno.0 as usize;
        proc.flag &= !(SLOAD | SLOCK);

        // Clear time since last swapin / swapout
        proc.time = 0;

        if self.run_out != 0 {
            self.run_out = 0;
            let chan = (&self.run_out).channel_addr();
            self.wakeup_all(chan);
        }
    }

    pub fn wakeup_all(&mut self, chan: impl Channel) {
        for proc in &mut self.procs {
            if !proc.is_sleeping_on(chan.channel_addr()) {
                continue;
            }
            proc.set_run();
        }
    }

    fn find(&self, pid: u32) -> Option<&Box<Process>> {
        self.procs.iter().find(|p| p.pid == pid)
    }

    fn find_mut(&mut self, pid: u32) -> Option<&mut Box<Process>> {
        self.procs.iter_mut().find(|p| p.pid == pid)
    }

    fn kill_pgroup(&mut self, signal: Signal) -> KResult<()> {
        let curuid = Userspace::get().proc().uid;
        let curtty = Userspace::get().proc().tty;
        for proc in &mut self.procs {
            // Ignore #0
            if proc.pid == 0 {
                continue;
            }
            if proc.tty != curtty {
                continue;
            }
            // Permit non-root from sending signals to other user's processes
            if curuid != 0 && proc.uid != curuid {
                continue;
            }

            proc.raise(signal);
            return Ok(());
        }

        Err(PosixError::ESRCH)
    }

    pub fn kill(&mut self, pid: u32, signal: Signal) -> KResult<()> {
        if Userspace::get().proc().pid == pid {
            return Err(PosixError::EINVAL);
        }

        if pid == 0 {
            return self.kill_pgroup(signal);
        }

        let curuid = Userspace::get().proc().uid;
        let proc = self.find_mut(pid).ok_or(PosixError::ESRCH)?;

        if curuid != 0 && proc.uid != curuid {
            return Err(PosixError::EPERM);
        }

        proc.raise(signal);
        Ok(())
    }

    pub fn exec(&mut self, proc: &mut Process, path: &[u8], argv: &[NonNull<i8>]) -> KResult<()> {
        crate::println_info!("Process {} execing", proc.pid);
        let inode = FileManager
            .find(path, DirSearchMode::Open)?
            .ok_or(PosixError::ENOENT)?;

        const NEXEC: u32 = 10;
        while self.exe_cnt >= NEXEC {
            proc.sleep_kernel(&self.exe_cnt, EXPRI);
        }
        self.exe_cnt += 1;

        if !inode.has_access(InodeMode::IEXEC) || !inode.is_regular() {
            drop(inode);
            if self.exe_cnt >= NEXEC {
                let chan = (&self.exe_cnt).channel_addr();
                self.wakeup_all(chan);
            }
            self.exe_cnt -= 1;
            return Err(PosixError::ENOEXEC);
        }

        let mut parser = PEParser::new();
        if !parser.load(&inode) {
            return Err(PosixError::ENOEXEC);
        }

        let mem = &mut Userspace::get().mem;
        mem.text = parser.text;
        mem.text_len = parser.text_len;
        mem.data = parser.data;
        mem.data_len = parser.data_len;
        mem.stack_len = parser.stack_size;

        if mem.overflow() {
            return Err(PosixError::ENOMEM);
        }

        let args: Vec<CString> = argv
            .iter()
            .map(|argp| unsafe { CStr::from_ptr(argp.as_ptr()) })
            .map(|arg_str| arg_str.to_owned())
            .collect();

        proc.text.take();

        // For struct Userspace
        proc.expand(PAGE_SIZE);

        // TODO: shared texts
        let mut text = Text::new(inode.clone(), mem.text_len);
        let shared_text = false;
        proc.text = Some(text.clone());

        let newlen = PAGE_SIZE + mem.data_len + mem.stack_len;
        proc.expand(newlen);

        mem.establish_user(proc);

        parser.relocate(&inode, shared_text);

        if !shared_text {
            proc.flag |= SLOCK;
            global_buffer_manager().swap(
                text.disk_addr,
                text.addr().unwrap(),
                text.len_bytes,
                BufFlag::B_WRITE,
            );
            proc.flag &= !SLOCK;
        }

        let mut stack = Stack {
            sp: MemoryDescriptor::USER_SPACE_END as *mut usize,
        };

        // End all arguments with a null pointer.
        let mut argv = vec![0];

        // Push all arguments, reversed.
        for arg in args.into_iter().rev() {
            let argp = stack.push_string(arg.as_bytes_with_nul());
            argv.push(argp);
        }

        let argc = argv.len() - 1;
        let paddings = (4 - argv.len() % 4) % 4;
        for _ in 0..paddings {
            stack.push_word(0);
        }

        // `argv` is already in reverse order, push it in order will reverse it again.
        for argp in argv {
            stack.push_word(argp);
        }

        let sp = stack.push_word(argc);

        drop(inode);
        if self.exe_cnt >= NEXEC {
            let chan = (&self.exe_cnt).channel_addr();
            self.wakeup_all(chan);
        }
        self.exe_cnt -= 1;

        Userspace::get().clear_signal_handlers();

        // Check `go_userspace()`
        proc.ctx.ebx = parser.entry;
        proc.ctx.esp = (&raw const TEMPORARY_STACK[5]) as usize;
        proc.ctx.edi = sp;
        proc.ctx.eip = go_userspace as *const () as usize;

        Ok(())
    }

    pub fn wait(&mut self, proc: &mut Process) {
        crate::println_info!("Process {} finding dead son. They are:", proc.pid);
        loop {
            let mut has_child = false;
            for p in &mut self.procs {
                if p.ppid != proc.pid {
                    continue;
                }

                crate::println_info!("Process {} (Status: {:?})", p.pid, p.stat);
                has_child = true;

                if p.stat == ProcessState::SZOMB {
                    // wait() 系统调用返回子进程的 pid
                    Userspace::get().set_user_retval(p.pid);

                    // 清理僵尸进程
                    p.stat = ProcessState::SNULL;
                    p.pid = 0;
                    p.ppid = 0;
                    p.pending_signal = None;
                    p.flag = 0;

                    // greatbridf: don't consider child time accounting for now.
                    // maybe add them back later...

                    crate::println_info!("end wait");
                    return;
                }
            }

            if has_child {
                // 睡眠等待直至子进程结束
                crate::println_info!("wait until child process Exit!");
                let chan = (proc as *const Process) as usize;
                const PWAIT: u32 = 40;
                proc.sleep_user(chan, PWAIT);
                crate::println_info!("end sleep");
                continue;
            } else {
                // 不存在需要等待结束的子进程
                Userspace::get().set_error(PosixError::ECHILD);
                break;
            }
        }
    }

    pub fn get() -> &'static mut Self {
        static PROCESS_MANAGER: LazyLock<SuperCell<ProcessManager>> =
            LazyLock::new(|| SuperCell::new(ProcessManager::new()));

        PROCESS_MANAGER.with_mut(|scheduler| unsafe { &mut *&raw mut *scheduler })
    }

    pub fn wakeup_runout(&mut self) {
        let chan = (&self.run_out).channel_addr();
        self.wakeup_all(chan);
    }

    pub fn wakeup_runin(&mut self) {
        let chan = (&self.run_in).channel_addr();
        self.wakeup_all(chan);
    }

    pub fn wake_ppid(&mut self, ppid: u32) {
        let mut chan = None;
        for proc in &mut self.procs {
            if proc.pid != ppid {
                continue;
            }
            chan = Some(proc.as_ref().channel_addr());
        }

        self.wakeup_all(chan.unwrap());
    }

    pub fn reparent(&mut self, pid: u32) {
        for proc in &mut self.procs {
            if proc.ppid != pid {
                continue;
            }

            crate::println_info!("My:{} 's child {} passed to 1#process", pid, proc.ppid);

            proc.ppid = 1;
            if proc.stat == ProcessState::SSTOP {
                proc.set_run();
            }
        }
    }

    fn select(&mut self) -> &mut Process {
        assert_eq!(Userspace::get().proc().pid, 0);
        static LAST_IDX: AtomicUsize = AtomicUsize::new(0);

        loop {
            let mut pri = 256;
            let mut best = None;
            let last = LAST_IDX.load(Ordering::Relaxed);
            let total = self.procs.len();

            self.runrun = 0;
            for i in 0..total {
                let idx = (last + 1 + i) % total;
                let proc = &mut self.procs[idx];

                if proc.stat != ProcessState::SRUN
                    || (proc.flag & SLOAD) == 0
                    || (proc.flag & SSYS) != 0
                {
                    continue;
                }

                if proc.pri >= pri {
                    continue;
                }

                pri = proc.pri;
                best = Some(idx);
            }

            let Some(best) = best else {
                halt();
                continue;
            };

            self.switch_cnt = self.switch_cnt.wrapping_add(1);
            self.cur_pri = pri;

            LAST_IDX.store(best, Ordering::Relaxed);
            return &mut self.procs[best];
        }
    }

    pub fn switch(&mut self) {
        let me = Userspace::get().proc();
        let scheduler = &mut *self.procs[0];

        switch_user_struct(&scheduler);

        unsafe {
            let ctx = IrqGuard::disable_save();
            TaskContext::switch(&mut me.ctx, &mut scheduler.ctx);
        }
    }

    pub fn schedule() {
        loop {
            let pm = ProcessManager::get();
            let me = Userspace::get().proc();
            let selected = pm.select();

            if (selected.flag & SSWAP) != 0 {
                todo!("swap");
                selected.flag &= !SSWAP;
            }

            switch_user_struct(&selected);
            Userspace::get().mem.map_to_actual_pt(&selected);

            // 更新 TSS esp0，使得从用户态陷入内核态时使用目标进程的内核栈
            if let Some(ref kstack) = selected.kstack {
                set_tss_esp0(kstack.top() as u32);
            }

            unsafe {
                let ctx = IrqGuard::disable_save();
                TaskContext::switch(&mut me.ctx, &mut selected.ctx);
            }
        }
    }

    fn set_fork_return_context(child: &mut Process) {
        let regs = Userspace::get().ssav[0].0 as *const Registers;
        let ctx = Userspace::get().ssav[1].0 as *const TrapFrame;

        let regs = unsafe { &*regs };
        let ctx = unsafe { &*ctx };
        let mut stack = Stack {
            sp: child.kstack.as_ref().unwrap().top() as *mut usize,
        };

        stack.push_word(ctx.xss);
        stack.push_word(ctx.esp as usize);
        stack.push_word(ctx.eflags);
        stack.push_word(ctx.xcs);
        stack.push_word(ctx.eip);
        stack.push_word(regs.ecx);
        let top = stack.push_word(regs.edx);

        child.ctx.eip = Self::fork_ret as usize;
        child.ctx.esp = top;
        child.ctx.ebp = regs.ebp;
        child.ctx.ebx = regs.ebx;
        child.ctx.esi = regs.esi;
        child.ctx.edi = regs.edi;
    }

    #[unsafe(no_mangle)]
    #[unsafe(naked)]
    extern "C" fn fork_ret() -> ! {
        naked_asm!(
            "xor %eax, %eax",
            "pop %edx",
            "pop %ecx",
            "iret",
            options(att_syntax),
        )
    }

    pub fn fork(&mut self) -> u32 {
        // Create a reborrow
        let mut child = self.new_proc(Userspace::get().proc());

        // TODO: set the child's return value to 0
        // TODO: clear child's {c,}{s,u}time

        let pid = child.pid;
        Self::set_fork_return_context(&mut child);
        self.procs.push(child);
        pid
    }

    pub fn new_proc_compat(&mut self) -> u32 {
        let child = self.new_proc(unsafe { &mut *&raw mut *Userspace::get().proc() });

        let pid = child.pid;
        self.procs.push(child);
        pid
    }

    pub fn new_init_proc(&mut self) -> u32 {
        let mut child = self.new_proc(unsafe { &mut *&raw mut *Userspace::get().proc() });

        let pid = child.pid;
        Self::set_init_context(&mut child);
        self.procs.push(child);
        pid
    }

    pub fn setup_proc_zero(&mut self) {
        const PPDA_ADDR: usize = 0x400000 - 0x1000;

        // 0 号进程的内核栈在 main.cpp 的 main0() 中通过 KernelStack_new() 分配，
        // 0 号进程是调度器（SSYS），永远运行在内核态，不需要通过 TSS esp0 切换栈。
        let mut proc = Box::new(Process {
            uid: 0,
            pid: ProcessManager::assign_pid(),
            ppid: 0,
            addr: PPDA_ADDR,
            size: 0x1000,
            text: None,
            stat: ProcessState::SRUN,
            flag: SLOAD | SSYS,
            pri: 0,
            cpu: 0,
            nice: 0,
            time: 0,
            wchan: 0,
            pending_signal: None,
            tty: core::ptr::null(),
            sigmap: 0,
            pages: None,
            ctx: TaskContext::new(),
            kstack: None,
        });

        Userspace_init();
        Userspace::get().proc = &raw mut *proc;

        self.procs.push(proc);
    }

    pub fn recalc_pri(&mut self) {
        const SCHED_MAGIC: u32 = 20;
        const MAX_TIME: u32 = 127;
        const PUSER: i32 = 100;

        for proc in &mut self.procs {
            proc.time = (proc.time + 1).min(MAX_TIME);
            proc.cpu = proc.cpu.saturating_sub(SCHED_MAGIC);
            if proc.pri > PUSER {
                proc.set_pri();
            }
        }
    }
}

static TEMPORARY_STACK: [usize; 6] = [0; 6];
static SHELL_PATH: [u8; 11] = *b"/Shell.exe\0";

#[unsafe(naked)]
extern "C" fn exec_shell() {
    naked_asm!(
        "mov ${shell}, %ebx", // path
        "mov $11, %eax",      // execv
        "xor %ecx, %ecx",     // argc
        "xor %edx, %edx",     // argv
        "int $0x80",
        shell = sym SHELL_PATH,
        options(att_syntax),
    );
}

#[unsafe(naked)]
extern "C" fn go_init() {
    naked_asm!(
        "mov ${exec_shell}, %esi",
        "mov $0x800, %edi",
        "mov $0x10, %ecx",
        "rep movsl",
        "push $0x23",     // ss
        "push $0x800000", // esp
        "push $0x200",    // eflags = IF
        "push $0x1b",     // cs
        "push $0x800",    // eip = entry
        "iret",
        exec_shell = sym exec_shell,
        options(att_syntax),
    )
}

#[unsafe(naked)]
extern "C" fn go_userspace() {
    naked_asm!(
        "mov %ebx, %eax", // eax = entry
        "push $0x23",     // ss
        "push %edi",      // esp
        "push $0x200",    // eflags = IF
        "push $0x1b",     // cs
        "push $0x10",        // eip = runtime
        "xor %ebx, %ebx",
        "xor %ecx, %ecx",
        "xor %edx, %edx",
        "xor %esi, %esi",
        "xor %edi, %edi",
        "xor %ebp, %ebp",
        "iret",
        options(att_syntax),
    )
}

fn halt() {
    unsafe {
        core::arch::asm!("hlt");
    }
}

struct Stack {
    sp: *mut usize,
}

impl Stack {
    fn top(&self) -> usize {
        self.sp as usize
    }

    /// Push a word to the top, and return the stack pointer after the push.
    fn push_word(&mut self, val: usize) -> usize {
        self.sp = self.sp.wrapping_sub(1);
        unsafe {
            self.sp.write(val);
        }

        self.top()
    }

    /// Push a null-terminated string to the top, align the stack to 16 bytes,
    /// and return the pointer to the string.
    fn push_string(&mut self, string: &[u8]) -> usize {
        self.sp = self.sp.wrapping_byte_sub(string.len());
        let addr = self.align16();

        unsafe {
            self.sp
                .cast::<u8>()
                .copy_from_nonoverlapping(string.as_ptr(), string.len());
        }

        addr
    }

    /// Align the stack to 16 bytes, and return the previous stack pointer.
    fn align16(&mut self) -> usize {
        let ret = self.top();
        self.sp = (self.top() & !0xf) as *mut _;

        ret
    }
}

define_class_compat! {impl ProcessManager {
    /// Wait() 的 Rust 实现 - 父进程等待子进程结束
    pub fn wait() {
        let pm = ProcessManager::get();
        let proc = Userspace::get().proc();
        pm.wait(proc);
    }

    /// 唤醒所有因 chan 而睡眠的进程（供 C++ 调用）
    pub fn wakeup_all_chan(chan: usize) {
        ProcessManager::get().wakeup_all(chan);
    }

    /// 唤醒等待 run_in 的进程（供 C++ 调用）
    pub fn wakeup_runin() {
        ProcessManager::get().wakeup_runin();
    }

    /// 时钟中断每秒末尾调用：更新所有进程的 p_time, p_cpu, p_pri
    pub fn clock_tick_processes(schmag: u32) {
        let pm = ProcessManager::get();
        for proc in &mut pm.procs {
            if proc.stat == ProcessState::SNULL {
                continue;
            }

            proc.time = core::cmp::min(proc.time + 1, 127);

            if proc.cpu > schmag {
                proc.cpu -= schmag;
            } else {
                proc.cpu = 0;
            }

            if proc.pri > 100 {
                // PUSER = 100
                proc.set_pri();
            }
        }
    }

    /// Ctrl+C 信号处理：向所有 pid > 1 的进程发送 SIGINT
    pub fn signal_ctrl_c() {
        let pm = ProcessManager::get();
        for proc in &mut pm.procs {
            if proc.pid > 1 {
                proc.raise(Signal::SIGINT);
            }
        }
    }

    /// Swtch() 的 Rust 实现 - 进程切换
    pub fn swtch() -> i32 {
        ProcessManager::get().switch();
        0
    }

    pub fn new_init_proc() -> i32 {
        let pm = ProcessManager::get();
        pm.new_init_proc() as i32
    }

    /// XSwap() 的 Rust 实现 - 进程换出
    pub fn xswap(proc: *mut Process, free_mem: bool, size: i32) {
        let pm = ProcessManager::get();
        let proc = unsafe { &mut *proc };
        let swap_len = if size == 0 {
            None
        } else {
            NonZero::new(size as usize)
        };
        pm.send_to_swap(proc, free_mem, swap_len);
    }

    /// Select() 的 Rust 实现 - 选择最适合运行的进程
    pub fn select() -> *mut Process {
        let pm = ProcessManager::get();
        let proc = pm.select();
        proc as *mut Process
    }
}}
