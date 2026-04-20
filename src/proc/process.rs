use kernel_macros::define_class_compat;

use crate::{compat::compat_user_exit, constants::{PosixError, SIGMAX, Signal}, serial::KResult, user::Userspace};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessState {
    SNULL = 0,
    SSLEEP = 1,
    SWAIT = 2,
    SRUN = 3,
    SIDL = 4,
    SZOMB = 5,
    SSTOP = 6,
}

#[repr(C)]
struct TaskContext {
    eip: usize,
    xcs: usize,
    eflags: usize,
    esp: *mut usize,
    xss: usize,
}

pub struct Text;
pub struct Terminal;

#[repr(C)]
pub struct Process {
    uid: u16,
    pid: u32,
    ppid: u32,

    addr: usize,
    size: u32,
    textp: *mut Text,
    stat: ProcessState,
    flag: u32,

    pri: u32,
    cpu: u32,
    nice: i32,
    time: u32,

    wchan: usize,

    pending_signal: Option<Signal>,
    tty: *const Terminal,
    sigmap: usize,
}

trait KResultExt {
    fn pass_to_user(self);
}

trait NativeWord {
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

    pub fn process_signal(&mut self, context: &mut TaskContext) {
        let Some(pending) = self.pending_signal.take() else {
            crate::println_warn!("Signal UNKNOWN");
            compat_user_exit();
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
        extern "C" {
            fn compat_set_run(proc: &mut Process);
        }

        unsafe {
            compat_set_run(self);
        }
    }

    pub fn raise(&mut self, signal: u32) -> KResult<()> {
        let signal = Signal::try_from(signal)?;

        crate::println_info!("{signal:?} triggered");

        // ???
        if signal == Signal::SIGKILL {
            return Ok(());
        }

        self.pending_signal = Some(signal);

        if self.pri > Self::PUSER {
            self.pri = Self::PUSER;
        }

        if self.stat == ProcessState::SWAIT {
            self.set_run();
        }

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
}

define_class_compat! {impl Process {
    pub fn send_signal(&mut self) {
        let signal = Userspace::get().args[0] as u32;
        let func = Userspace::get().args[1];

        this.send_signal(signal, func).pass_to_user();
    }

    pub fn process_signal(&mut self, context: &mut TaskContext) {
        this.process_signal(context);
    }

    pub fn should_process(&mut self) -> bool {
        this.should_process()
    }

    pub fn raise(&mut self, signal: u32) {
        this.raise(signal).pass_to_user();
    }

    pub fn set_nice(&mut self) {
        let nice = Userspace::get().args[0] as i32;
        this.set_nice(nice);
    }
}}
