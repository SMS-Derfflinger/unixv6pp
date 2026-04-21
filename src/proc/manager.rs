use core::{num::NonZero, sync::atomic::{AtomicU32, AtomicUsize, Ordering}};

use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};

use crate::{constants::{PosixError, Signal}, dev::{buffer::{BufFlag, PhysicalBlock}, buffer_manager::global_buffer_manager}, mm::USER_PAGE_MANAGER, proc::{Process, process::{ProcessState, Text}, wakeup_all}, serial::KResult, sync::SpinExt, user::Userspace};

static NEXT_PID: AtomicU32 = AtomicU32::new(0);

pub struct ProcessImage();

pub struct ProcessManager {
    procs: Vec<Box<Process>>,
    texts: [Option<Arc<Text>>; Self::NTEXT],

    cur_pri: u32,
    runrun: u32,
    run_in: u32,
    run_out: u32,
    exe_cnt: u32,
    switch_cnt: u32,
}

pub const SLOAD: u32 = (1 << 0);
pub const SLOCK: u32 = (1 << 2);

impl Process {
    pub fn new_from(pid: u32, parent: &Self) -> Box<Self> {
        unsafe {
            if let Some(text) = parent.textp.as_mut() {
                text.get();
            }
        }

        Box::new(Self {
            uid: parent.uid,
            pid,
            ppid: parent.pid,
            addr: 0,
            size: parent.size,
            textp: parent.textp,
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
            pages: None, // TODO
        })
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
            texts: [const { None }; Self::NTEXT],
            cur_pri: 0,
            runrun: 0,
            run_in: 0,
            run_out: 0,
            exe_cnt: 0,
            switch_cnt: 0,
        }
    }

    pub fn new_proc(&mut self, parent: &Process) -> Box<Process> {
        let proc = Process::new_from(Self::assign_pid(), parent);

        // TODO: get all open files
        // TODO: increase cwd inode refcount

        proc
    }

    pub fn raise(&mut self, tty: *const Terminal, signal: Signal) {
        for proc in &self.procs {
            if proc.tty != tty {
                continue;
            }

            proc.raise(signal);
        }
    }

    pub fn send_to_swap(
        &mut self, proc: &mut Process, do_free: bool, swap_len: Option<NonZero<usize>>,
    ) {
        let swap_len = swap_len
            .map(|l| l.get())
            .unwrap_or(proc.size as usize);

        let blkno = compat_alloc_swap(proc.size);

        if let Some(text) = unsafe { proc.textp.as_mut() } {
            text.put_mem();
        }

        proc.flag |= SLOCK;
        global_buffer_manager()
            .swap(PhysicalBlock(blkno), proc.addr, swap_len, BufFlag::B_WRITE)
            .expect("Swap I/O Error");

        if do_free {
            let pages = proc.pages.take().unwrap();
            unsafe {
                USER_PAGE_MANAGER.lock().dealloc(pages);
            }
        }

        // (flag & SLOAD) => addr == blkno
        proc.addr = blkno as usize;
        proc.flag &= !(SLOAD | SLOCK);

        // Clear time since last swapin / swapout
        proc.time = 0;

        if self.run_out != 0 {
            self.run_out = 0;
            wakeup_all(&self.run_out);
        }
    }

    pub fn wakeup_all(&mut self, chan: usize) {
        for proc in &mut self.procs {
            if !proc.is_sleeping_on(chan) {
                continue;
            }
            proc.set_run();
        }
    }

    fn find(&self, pid: u32) -> Option<&Box<Process>> {
        self.procs.iter().find(|p| p.pid == pid)
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
        let proc = self.find(pid).ok_or(PosixError::ESRCH)?;

        if curuid != 0 && proc.uid != curuid {
            return Err(PosixError::EPERM);
        }

        proc.raise(signal);
        Ok(())
    }

    fn select(&mut self) -> &mut Process {
        assert_eq!(Userspace::get().proc().pid, 0);
        static LAST_IDX: AtomicUsize = AtomicUsize::new(0);

        loop {
            let mut pri = 256;
            let mut best = None;
            let mut idx = 0;
            let last = LAST_IDX.load(Ordering::Relaxed);
            let total = self.procs.len();

            self.runrun = 0;
            for i in 0..total {
                idx = (last + i) % total;
                let proc = &mut self.procs[idx];

                if proc.stat != ProcessState::SRUN || (proc.flag & SLOAD) != 0 {
                    continue;
                }

                if proc.pri >= pri {
                    continue;
                }

                pri = proc.pri;
                best = Some(proc);
            }

            let Some(best) = best else {
                halt();
                continue;
            };

            self.switch_cnt = self.switch_cnt.wrapping_add(1);
            self.cur_pri = pri;

            LAST_IDX.store(idx, Ordering::Relaxed);
            return &mut *best;
        }
    }
}

fn compat_alloc_swap(len: u32) -> u32 { todo!() }
fn halt() {
    unsafe {
        core::arch::asm!("hlt");
    }
}
