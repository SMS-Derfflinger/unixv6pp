use core::{
    arch::naked_asm,
    ffi::CStr,
    mem::size_of,
    num::NonZero,
    ptr::NonNull,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, ffi::CString, vec, vec::Vec};
use eonix_mm::address::{Addr, PAddr};
use eonix_sync_base::LazyLock;
use riscv::register::sstatus::{self, SPP};

use crate::{
    constants::{PosixError, Signal},
    dev::{buffer::BufFlag, buffer_manager::global_buffer_manager},
    fs::{DirSearchMode, FileManager, InodeMode, InodeRefExt},
    interrupt::context::{Registers, TrapContext},
    loader::ELFParser,
    machine::{asm, switch_user_struct},
    mm::{phys_copy, swap_alloc, KernelStack, PAGE_SIZE, USER_PAGE_MANAGER},
    proc::{
        context::TaskContext,
        process::{ProcessState, Terminal, Text},
        Channel, Process, EXPRI,
    },
    serial::KResult,
    sync::{IrqGuard, SpinExt, SuperCell},
    user::{MemoryDescriptor, Userspace},
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
            trap_context: TrapContext::new(),
            ctx: TaskContext::new(),
            kstack,
        });

        child.init_kernel_context(None);

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

    pub fn bind_current_trap_context(&mut self) {
        let proc = Userspace::get().proc();
        asm::write_sscratch(proc.trap_context_ptr() as usize);
    }

    fn switch_to(current: &mut Process, next: &mut Process) {
        switch_user_struct(next);
        Userspace::get().mem.map_to_actual_pt(next);
        Userspace::get().proc = &raw mut *next;
        asm::write_sscratch(next.trap_context_ptr() as usize);

        unsafe {
            let _ctx = IrqGuard::disable_save();
            TaskContext::switch(&mut current.ctx, &mut next.ctx);
        }
    }

    fn set_init_context(child: &mut Process) {
        child.init_kernel_context(Some(go_init as usize));
    }

    fn prepare_user_context(
        proc: &mut Process,
        entry: usize,
        user_sp: usize,
        argc: usize,
        argv: usize,
    ) {
        let context = unsafe { &mut *proc.trap_context_ptr() };
        *context = TrapContext::new();

        let mut user_sstatus = sstatus::read();
        user_sstatus.set_spp(SPP::User);
        user_sstatus.set_spie(true);

        context.sstatus = user_sstatus;
        context.sepc = entry;
        context.kernel_sp = proc.kstack.top().addr().get();
        context.regs.sp = user_sp;
        context.regs.a0 = argc;
        context.regs.a1 = argv;
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
                phys_copy(cur_addr, to_addr, 1);
                cur_addr = cur_addr + 1;
                to_addr = to_addr + 1;
            }
            child.pages = Some(pages);
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
            if proc.pid <= 1 {
                continue;
            }

            // if proc.tty != tty {
            //     continue;
            // }

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

        let blkno = swap_alloc(proc.size as usize);

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
        #[cfg(feature = "debug_irq")]
        crate::println_debug!("waking up chan={:#x}", chan.channel_addr());

        for proc in &mut self.procs {
            if !proc.is_sleeping_on(chan.channel_addr()) {
                continue;
            }

            #[cfg(feature = "debug_irq")]
            crate::println_debug!("waking up chan={:#x} pid {:2}", chan.channel_addr(), proc.pid);
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

        let mut parser = ELFParser::new();
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
            .map(|argp| unsafe { CStr::from_ptr(argp.as_ptr() as *const u8) })
            .map(|arg_str| arg_str.to_owned())
            .collect();

        proc.text.take();

        // For struct Userspace
        proc.expand(PAGE_SIZE);

        // TODO: shared texts
        let mut text = Text::new(inode.clone(), mem.text_len);
        let shared_text = false;
        proc.text = Some(text.clone());

        let newlen = mem.resident_len();
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

        let argv_ptr = stack.top();

        // Pad with two zeros to have the stack aligned to 16 bytes
        stack.push_word(0);
        stack.push_word(0);
        stack.push_word(argv_ptr);
        let sp = stack.push_word(argc);

        drop(inode);
        if self.exe_cnt >= NEXEC {
            let chan = (&self.exe_cnt).channel_addr();
            self.wakeup_all(chan);
        }
        self.exe_cnt -= 1;

        Userspace::get().clear_signal_handlers();
        Self::prepare_user_context(proc, parser.entry, sp, argc, argv_ptr);

        Ok(())
    }

    pub fn wait(&mut self, proc: &mut Process) -> KResult<u32> {
        crate::println_info!(
            "[{pid:2}] Process {pid} finding dead son. They are:",
            pid = proc.pid
        );

        loop {
            let mut has_child = false;
            for p in &mut self.procs {
                if p.ppid != proc.pid {
                    continue;
                }

                crate::println_info!(
                    "[{pid:2}] Process {} (Status: {:?})",
                    p.pid,
                    p.stat,
                    pid = proc.pid,
                );

                has_child = true;

                if p.stat == ProcessState::SZOMB {
                    // wait() 系统调用返回子进程的 pid
                    let pid = p.pid;

                    // 清理僵尸进程
                    p.stat = ProcessState::SNULL;
                    p.pid = 0;
                    p.ppid = 0;
                    p.pending_signal = None;
                    p.flag = 0;

                    // greatbridf: don't consider child time accounting for now.
                    // maybe add them back later...

                    crate::println_info!("[{pid:2}] end wait", pid = proc.pid);
                    return Ok(pid);
                }
            }

            if has_child {
                // 睡眠等待直至子进程结束
                crate::println_info!("[{pid:2}] wait until child process Exit!", pid = proc.pid);
                let chan = (proc as *const Process) as usize;
                const PWAIT: u32 = 40;
                proc.sleep_user(chan, PWAIT)?;
                crate::println_info!("[{pid:2}] end sleep", pid = proc.pid);
                continue;
            } else {
                // 不存在需要等待结束的子进程
                return Err(PosixError::ECHILD);
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

            crate::println_info!(
                "[{pid:2}] exit: child {} passed to 1#process",
                proc.pid,
                pid = pid,
            );

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
        Self::switch_to(me, scheduler);
    }

    pub fn schedule() -> ! {
        loop {
            let pm = ProcessManager::get();
            let me = Userspace::get().proc();
            let selected = pm.select();

            #[cfg(feature = "debug_scheduler")]
            crate::println_debug!("[{pid:2}] selected", pid = selected.pid);

            if (selected.flag & SSWAP) != 0 {
                todo!("swap");
                selected.flag &= !SSWAP;
            }

            // RISC-V 不使用 x86 的 TSS esp0。
            // 这里通过切换当前进程、u-area 窗口和 sscratch，
            // 让后续 trap 进入目标进程自己的内核态上下文。
            Self::switch_to(me, &mut *selected);
        }
    }

    fn set_fork_return_context(child: &mut Process) {
        let parent_context = Userspace::get().ssav[1].0 as *const TrapContext;
        let parent_context = unsafe { &*parent_context };

        crate::println_info!(
            "fork ctx: parent pid={} sepc={:#x} ra={:#x} sp={:#x}",
            Userspace::get().proc().pid,
            parent_context.sepc,
            parent_context.regs.ra,
            parent_context.regs.sp
        );

        child.init_kernel_context(Some(fork_ret as *const () as usize));
        let child_context = unsafe { &mut *child.trap_context_ptr() };
        *child_context = *parent_context;
        child_context.kernel_sp = child.kstack.top().addr().get();
        child_context.set_return_value(0);
    }

    pub fn fork(&mut self) -> u32 {
        let mut child = self.new_proc(Userspace::get().proc());

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

        // 0 号进程是调度器（SSYS），永远运行在内核态。
        // RISC-V 下不再依赖 TSS esp0，而是通过 sscratch 绑定当前 trap 上下文。
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
            trap_context: TrapContext::new(),
            ctx: TaskContext::new(),
            kstack: KernelStack::new(),
        });
        proc.init_kernel_context(None);

        unsafe {
            Userspace::init();
        }
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

static SHELL_PATH: [u8; 11] = *b"/Shell.exe\0";

#[unsafe(naked)]
extern "C" fn go_userspace() -> ! {
    naked_asm!(
        "csrr  t0, sscratch",
        "ld    t1, {sepc}(t0)",
        "ld    t2, {sstatus}(t0)",
        "csrw  sepc, t1",
        "csrw  sstatus, t2",
        "ld    tp,   {tp}(t0)",
        "ld    ra,   {ra}(t0)",
        "ld    sp,   {sp}(t0)",
        "ld    gp,   {gp}(t0)",
        "ld    a0,   {a0}(t0)",
        "ld    a1,   {a1}(t0)",
        "ld    a2,   {a2}(t0)",
        "ld    a3,   {a3}(t0)",
        "ld    a4,   {a4}(t0)",
        "ld    t1,   {t1}(t0)",
        "ld    a5,   {a5}(t0)",
        "ld    a6,   {a6}(t0)",
        "ld    a7,   {a7}(t0)",
        "ld    t3,   {t3}(t0)",
        "ld    t4,   {t4}(t0)",
        "ld    t5,   {t5}(t0)",
        "ld    t2,   {t2}(t0)",
        "ld    t6,   {t6}(t0)",
        "ld    s0,   {s0}(t0)",
        "ld    s1,   {s1}(t0)",
        "ld    s2,   {s2}(t0)",
        "ld    s3,   {s3}(t0)",
        "ld    s4,   {s4}(t0)",
        "ld    s5,   {s5}(t0)",
        "ld    s6,   {s6}(t0)",
        "ld    s7,   {s7}(t0)",
        "ld    s8,   {s8}(t0)",
        "ld    s9,   {s9}(t0)",
        "ld   s10,  {s10}(t0)",
        "ld   s11,  {s11}(t0)",
        "ld    t0,   {t0}(t0)",
        "sret",
        tp = const Registers::OFFSET_TP,
        ra = const Registers::OFFSET_RA,
        sp = const Registers::OFFSET_SP,
        gp = const Registers::OFFSET_GP,
        a0 = const Registers::OFFSET_A0,
        a1 = const Registers::OFFSET_A1,
        a2 = const Registers::OFFSET_A2,
        a3 = const Registers::OFFSET_A3,
        a4 = const Registers::OFFSET_A4,
        t1 = const Registers::OFFSET_T1,
        a5 = const Registers::OFFSET_A5,
        a6 = const Registers::OFFSET_A6,
        a7 = const Registers::OFFSET_A7,
        t3 = const Registers::OFFSET_T3,
        t4 = const Registers::OFFSET_T4,
        t5 = const Registers::OFFSET_T5,
        t2 = const Registers::OFFSET_T2,
        t6 = const Registers::OFFSET_T6,
        s0 = const Registers::OFFSET_S0,
        s1 = const Registers::OFFSET_S1,
        s2 = const Registers::OFFSET_S2,
        s3 = const Registers::OFFSET_S3,
        s4 = const Registers::OFFSET_S4,
        s5 = const Registers::OFFSET_S5,
        s6 = const Registers::OFFSET_S6,
        s7 = const Registers::OFFSET_S7,
        s8 = const Registers::OFFSET_S8,
        s9 = const Registers::OFFSET_S9,
        s10 = const Registers::OFFSET_S10,
        s11 = const Registers::OFFSET_S11,
        t0 = const Registers::OFFSET_T0,
        sstatus = const TrapContext::OFFSET_SSTATUS,
        sepc = const TrapContext::OFFSET_SEPC,
    )
}

extern "C" fn go_init() -> ! {
    let proc = Userspace::get().proc() as *mut Process;
    let argv: [NonNull<i8>; 0] = [];
    let path = &SHELL_PATH[..SHELL_PATH.len() - 1];

    if let Err(err) = ProcessManager::get().exec(unsafe { &mut *proc }, path, &argv) {
        panic!("failed to exec init shell: {:?}", err);
    }

    go_userspace();
}

#[unsafe(naked)]
extern "C" fn fork_ret() -> ! {
    naked_asm!(
        "csrr  t0, sscratch",
        "sd    zero, {a0}(t0)",
        "j     {go_userspace}",
        a0 = const Registers::OFFSET_A0,
        go_userspace = sym go_userspace,
    )
}

fn halt() {
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
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
        self.align16();

        unsafe {
            self.sp
                .cast::<u8>()
                .copy_from_nonoverlapping(string.as_ptr(), string.len());
        }

        self.top()
    }

    /// Align the stack to 16 bytes, and return the previous stack pointer.
    fn align16(&mut self) -> usize {
        let ret = self.top();
        self.sp = (self.top() & !0xf) as *mut _;
        ret
    }
}
