#[repr(u32)]
enum ProcessState {
    SNULL = 0,
    SSLEEP = 1,
    SWAIT = 2,
    SRUN = 3,
    SIDL = 4,
    SZOMB = 5,
    SSTOP = 6,
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
    nice: u32,
    time: u32,

    wchan: usize,

    sig: u32,
    tty: *const Terminal,
    sigmap: usize,
}

impl Process {
    pub fn setuid(&mut self, uid: u16) {
        self.uid = uid;
    }
}
