use eonix_sync_base::LazyLock;

use crate::{constants::Signal, proc::ProcessManager, serial, sync::SuperCell};

const TTY_BUF_SIZE: usize = 512;
const CANBSIZ: usize = 256;

const CERASE: u8 = b'\x08';
const CEOT: u8 = 0x03;
const CKILL: u8 = 0x15;

const TTHIWAT: usize = 512;

const ECHO: u32 = 0x08;
const CRMOD: u32 = 0x10;
const RAW: u32 = 0x20;
const LCASE: u32 = 0x04;

const ISOPEN: u32 = 0x01;
const CARR_ON: u32 = 0x02;

pub const TTIPRI: u32 = 10;

#[derive(Clone, Copy)]
struct TtyQueue {
    head: usize,
    tail: usize,
    buf: [u8; TTY_BUF_SIZE],
}

impl TtyQueue {
    const fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            buf: [0; TTY_BUF_SIZE],
        }
    }

    fn get_char(&mut self) -> Option<u8> {
        if self.head == self.tail {
            return None;
        }

        let ch = self.buf[self.tail];
        self.tail = (self.tail + 1) & (TTY_BUF_SIZE - 1);
        Some(ch)
    }

    fn put_char(&mut self, ch: u8) {
        self.buf[self.head] = ch;
        self.head = (self.head + 1) & (TTY_BUF_SIZE - 1);
    }

    fn char_num(&self) -> usize {
        self.head.wrapping_sub(self.tail) & (TTY_BUF_SIZE - 1)
    }

    fn clear(&mut self) {
        while self.get_char().is_some() {}
    }
}

pub struct Tty {
    rawq: TtyQueue,
    canq: TtyQueue,
    outq: TtyQueue,
    flags: u32,
    delct: usize,
    erase: u8,
    kill: u8,
    state: u32,
    canonb: [u8; CANBSIZ],
}

impl Tty {
    pub const fn new() -> Self {
        Self {
            rawq: TtyQueue::new(),
            canq: TtyQueue::new(),
            outq: TtyQueue::new(),
            flags: 0,
            delct: 0,
            erase: CERASE,
            kill: CKILL,
            state: 0,
            canonb: [0; CANBSIZ],
        }
    }

    pub fn open(&mut self) {
        if self.state & ISOPEN == 0 {
            self.state = ISOPEN | CARR_ON;
            self.flags = ECHO | CRMOD;
            self.erase = CERASE;
            self.kill = CKILL;
        }
    }

    pub fn read_available(&mut self, out: &mut [u8]) -> Option<usize> {
        if self.state & CARR_ON == 0 {
            return Some(0);
        }

        if self.canq.char_num() == 0 && !self.canon() {
            return None;
        }

        let mut nread = 0;
        while nread < out.len() {
            let Some(ch) = self.canq.get_char() else {
                break;
            };

            out[nread] = ch;
            nread += 1;
        }

        Some(nread)
    }

    pub fn read_wait_channel(&self) -> usize {
        &self.rawq as *const TtyQueue as usize
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        if self.state & CARR_ON == 0 {
            return 0;
        }

        let mut nwritten = 0;
        for &ch in data {
            if self.outq.char_num() > TTHIWAT {
                self.start();
            }

            self.output(ch);
            nwritten += 1;
        }

        self.start();
        nwritten
    }

    pub fn input(&mut self, mut ch: u8) {
        if ch == b'\r' && self.flags & CRMOD != 0 {
            ch = b'\n';
        }

        if self.flags & LCASE != 0 && ch.is_ascii_uppercase() {
            ch = ch.to_ascii_lowercase();
        }

        self.rawq.put_char(ch);

        if self.flags & RAW != 0 || ch == b'\n' || ch == CEOT {
            self.rawq.put_char(0x07);
            self.delct += 1;
            ProcessManager::get().wakeup_all(self.read_wait_channel());
        }

        if ch == CEOT {
            ProcessManager::get().raise(core::ptr::null(), Signal::SIGINT);
        }

        if self.flags & ECHO != 0 {
            self.output(ch);
            self.start();
        }
    }

    pub fn flush(&mut self) {
        self.canq.clear();
        self.outq.clear();
        self.rawq.clear();
        self.delct = 0;
    }

    fn output(&mut self, ch: u8) {
        if ch == CEOT && self.flags & RAW == 0 {
            return;
        }

        if ch == b'\n' && self.flags & CRMOD != 0 {
            self.output(b'\r');
        }

        if ch != 0 {
            self.outq.put_char(ch);
        }
    }

    fn start(&mut self) {
        while let Some(ch) = self.outq.get_char() {
            serial::serial_write_bytes(core::iter::once(ch));
        }
    }

    fn canon(&mut self) -> bool {
        if self.delct == 0 {
            return false;
        }

        let mut len = 0;
        while let Some(ch) = self.rawq.get_char() {
            if ch == 0x07 {
                self.delct -= 1;
                break;
            }

            if self.flags & RAW == 0 {
                let escaped = len > 0 && self.canonb[len - 1] == b'\\';

                if !escaped {
                    if ch == self.erase {
                        len = len.saturating_sub(1);
                        continue;
                    }

                    if ch == self.kill {
                        len = 0;
                        continue;
                    }

                    if ch == CEOT {
                        continue;
                    }
                }
            }

            if len < CANBSIZ {
                self.canonb[len] = ch;
                len += 1;
            } else {
                break;
            }
        }

        for i in 0..len {
            self.canq.put_char(self.canonb[i]);
        }

        true
    }
}

static CONSOLE_TTY: LazyLock<SuperCell<Tty>> = LazyLock::new(|| SuperCell::new(Tty::new()));

pub fn console_tty() -> &'static SuperCell<Tty> {
    &CONSOLE_TTY
}

pub fn tty_input_byte(ch: u8) {
    console_tty().with_mut(|tty| tty.input(ch));
}

pub fn tty_flush() {
    console_tty().with_mut(Tty::flush);
}
