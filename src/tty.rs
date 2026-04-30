use alloc::collections::VecDeque;
use bitflags::bitflags;
use eonix_spin::{Spin, SpinGuard};
use eonix_sync_base::LazyLock;

use crate::{constants::Signal, proc::ProcessManager, serial, sync::IrqContext};

const BUFFER_SIZE: usize = 4096;
const TAB_WIDTH: usize = 8;

const NCCS: usize = 19;

const VINTR: usize = 0;
const VQUIT: usize = 1;
const VERASE: usize = 2;
const VKILL: usize = 3;
const VEOF: usize = 4;
const VMIN: usize = 6;
const VSUSP: usize = 10;
const VEOL: usize = 11;
const VEOL2: usize = 16;

const ISOPEN: u32 = 0x01;
const CARR_ON: u32 = 0x02;

pub const TTIPRI: u32 = 10;

macro_rules! CTRL {
    ('A') => {
        0x01
    };
    ('B') => {
        0x02
    };
    ('C') => {
        0x03
    };
    ('D') => {
        0x04
    };
    ('E') => {
        0x05
    };
    ('F') => {
        0x06
    };
    ('G') => {
        0x07
    };
    ('H') => {
        0x08
    };
    ('I') => {
        0x09
    };
    ('J') => {
        0x0A
    };
    ('K') => {
        0x0B
    };
    ('L') => {
        0x0C
    };
    ('M') => {
        0x0D
    };
    ('N') => {
        0x0E
    };
    ('O') => {
        0x0F
    };
    ('P') => {
        0x10
    };
    ('Q') => {
        0x11
    };
    ('R') => {
        0x12
    };
    ('S') => {
        0x13
    };
    ('T') => {
        0x14
    };
    ('U') => {
        0x15
    };
    ('V') => {
        0x16
    };
    ('W') => {
        0x17
    };
    ('X') => {
        0x18
    };
    ('Y') => {
        0x19
    };
    ('Z') => {
        0x1A
    };
    ('\\') => {
        0x1c
    };
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct TermioIFlags: u16 {
        const IGNBRK = 0x0001;
        const BRKINT = 0x0002;
        const IGNPAR = 0x0004;
        const PARMRK = 0x0008;
        const INPCK = 0x0010;
        const ISTRIP = 0x0020;
        const INLCR = 0x0040;
        const IGNCR = 0x0080;
        const ICRNL = 0x0100;
        const IUCLC = 0x0200;
        const IXON = 0x0400;
        const IXANY = 0x0800;
        const IXOFF = 0x1000;
        const IMAXBEL = 0x2000;
        const IUTF8 = 0x4000;
    }

    #[derive(Clone, Copy, Debug)]
    pub struct TermioOFlags: u16 {
        const OPOST = 0x0001;
        const OLCUC = 0x0002;
        const ONLCR = 0x0004;
        const OCRNL = 0x0008;
        const ONOCR = 0x0010;
        const ONLRET = 0x0020;
        const OFILL = 0x0040;
        const OFDEL = 0x0080;
    }

    #[derive(Clone, Copy, Debug)]
    pub struct TermioLFlags: u16 {
        const ISIG = 0x0001;
        const ICANON = 0x0002;
        const XCASE = 0x0004;
        const ECHO = 0x0008;
        const ECHOE = 0x0010;
        const ECHOK = 0x0020;
        const ECHONL = 0x0040;
        const NOFLSH = 0x0080;
        const TOSTOP = 0x0100;
        const ECHOCTL = 0x0200;
        const ECHOPRT = 0x0400;
        const ECHOKE = 0x0800;
        const FLUSHO = 0x1000;
        const PENDIN = 0x4000;
        const IEXTEN = 0x8000;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Termios {
    iflag: TermioIFlags,
    oflag: TermioOFlags,
    lflag: TermioLFlags,
    cc: [u8; NCCS],
}

impl Termios {
    pub fn veof(&self) -> u8 {
        self.cc[VEOF]
    }

    pub fn veol(&self) -> u8 {
        self.cc[VEOL]
    }

    pub fn veol2(&self) -> u8 {
        self.cc[VEOL2]
    }

    pub fn vintr(&self) -> u8 {
        self.cc[VINTR]
    }

    pub fn vquit(&self) -> u8 {
        self.cc[VQUIT]
    }

    pub fn vsusp(&self) -> u8 {
        self.cc[VSUSP]
    }

    pub fn verase(&self) -> u8 {
        self.cc[VERASE]
    }

    pub fn vkill(&self) -> u8 {
        self.cc[VKILL]
    }

    pub fn echo(&self) -> bool {
        self.lflag.contains(TermioLFlags::ECHO)
    }

    pub fn echoe(&self) -> bool {
        self.lflag.contains(TermioLFlags::ECHOE)
    }

    pub fn echoctl(&self) -> bool {
        self.lflag.contains(TermioLFlags::ECHOCTL)
    }

    pub fn echoke(&self) -> bool {
        self.lflag.contains(TermioLFlags::ECHOKE)
    }

    pub fn echok(&self) -> bool {
        self.lflag.contains(TermioLFlags::ECHOK)
    }

    pub fn echonl(&self) -> bool {
        self.lflag.contains(TermioLFlags::ECHONL)
    }

    pub fn isig(&self) -> bool {
        self.lflag.contains(TermioLFlags::ISIG)
    }

    pub fn icanon(&self) -> bool {
        self.lflag.contains(TermioLFlags::ICANON)
    }

    pub fn iexten(&self) -> bool {
        self.lflag.contains(TermioLFlags::IEXTEN)
    }

    pub fn igncr(&self) -> bool {
        self.iflag.contains(TermioIFlags::IGNCR)
    }

    pub fn icrnl(&self) -> bool {
        self.iflag.contains(TermioIFlags::ICRNL)
    }

    pub fn inlcr(&self) -> bool {
        self.iflag.contains(TermioIFlags::INLCR)
    }

    pub fn noflsh(&self) -> bool {
        self.lflag.contains(TermioLFlags::NOFLSH)
    }

    pub fn new_standard() -> Self {
        let cc = core::array::from_fn(|idx| match idx {
            VINTR => CTRL!('C'),
            VQUIT => CTRL!('\\'),
            VERASE => 0x7f,
            VKILL => CTRL!('U'),
            VEOF => CTRL!('D'),
            VSUSP => CTRL!('Z'),
            VMIN => 1,
            _ => 0,
        });

        Self {
            iflag: TermioIFlags::ICRNL | TermioIFlags::IXOFF,
            oflag: TermioOFlags::OPOST | TermioOFlags::ONLCR,
            lflag: TermioLFlags::ISIG
                | TermioLFlags::ICANON
                | TermioLFlags::ECHO
                | TermioLFlags::ECHOE
                | TermioLFlags::ECHOK
                | TermioLFlags::ECHOCTL
                | TermioLFlags::ECHOKE
                | TermioLFlags::IEXTEN,
            cc,
        }
    }
}

struct TerminalInner {
    termio: Termios,
    buffer: VecDeque<u8>,
    line_len: usize,
}

pub struct Tty {
    inner: TerminalInner,
    state: u32,
    column: usize,
}

impl Tty {
    pub fn new() -> Self {
        Self {
            inner: TerminalInner {
                termio: Termios::new_standard(),
                buffer: VecDeque::with_capacity(BUFFER_SIZE),
                line_len: 0,
            },
            state: 0,
            column: 0,
        }
    }

    pub fn open(&mut self) {
        if self.state & ISOPEN == 0 {
            self.state = ISOPEN | CARR_ON;
            self.inner.termio = Termios::new_standard();
        }
    }

    fn clear_read_buffer(&mut self, inner: &mut TerminalInner) {
        inner.buffer.clear();
        inner.line_len = 0;
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        for &ch in data {
            self.write_output(ch);
        }
        data.len()
    }

    fn erase(&mut self, inner: &mut TerminalInner, echo: bool) -> Option<u8> {
        let back = inner.buffer.back().copied();
        match back {
            None => return None,
            Some(b'\n') => return None,
            Some(back) if back == inner.termio.veof() => return None,
            Some(back) if back == inner.termio.veol() => return None,
            Some(back) if back == inner.termio.veol2() => return None,
            Some(0) => return None,
            _ => {}
        }

        let back = inner.buffer.pop_back();
        inner.line_len = inner.line_len.saturating_sub(1);

        if echo && inner.termio.echo() && inner.termio.echoe() {
            self.erase_width(1);
        }

        back
    }

    fn echo_char(&mut self, inner: &mut TerminalInner, ch: u8) {
        match ch {
            b'\t' | b'\n' | b'\x11' | b'\x13' | 32.. => self.write_output(ch),
            _ if !inner.termio.echo() => self.write_output(ch),
            _ if !inner.termio.echoctl() => self.write_output(ch),
            _ if !inner.termio.iexten() => self.write_output(ch),
            _ => {
                self.write_output(b'^');
                self.write_output(ch + 0x40);
            }
        }
    }

    fn signal(&mut self, inner: &mut TerminalInner, signal: Signal) {
        if !inner.termio.noflsh() {
            self.clear_read_buffer(inner);
        }
        ProcessManager::get().raise(core::ptr::null(), signal);
    }

    fn echo_and_signal(&mut self, inner: &mut TerminalInner, ch: u8, signal: Signal) {
        self.echo_char(inner, ch);
        self.signal(inner, signal);
    }

    fn do_commit_char(&mut self, inner: &mut TerminalInner, ch: u8) {
        if inner.termio.icanon() {
            if inner.line_len < BUFFER_SIZE {
                inner.line_len += 1;
            }
        }

        inner.buffer.push_back(ch);

        if inner.termio.echo() || (ch == b'\n' && inner.termio.echonl()) {
            self.echo_char(inner, ch);
        }

        if ch == b'\n' {
            inner.line_len = 0;
            self.wakeup_readers();
        } else if !inner.termio.icanon() {
            self.wakeup_readers();
        }
    }

    pub fn input(&mut self, ch: u8) {
        let mut inner = core::mem::replace(
            &mut self.inner,
            TerminalInner {
                termio: Termios::new_standard(),
                buffer: VecDeque::new(),
                line_len: 0,
            },
        );

        if ch == b'\t' {
            self.inner = inner;
            return;
        }

        if inner.termio.isig() {
            match ch {
                0xff => {}
                ch if ch == inner.termio.vintr() => {
                    self.echo_and_signal(&mut inner, ch, Signal::SIGINT);
                    self.inner = inner;
                    return;
                }
                ch if ch == inner.termio.vquit() => {
                    self.echo_and_signal(&mut inner, ch, Signal::SIGQUIT);
                    self.inner = inner;
                    return;
                }
                ch if ch == inner.termio.vsusp() => {
                    self.echo_and_signal(&mut inner, ch, Signal::SIGTSTP);
                    self.inner = inner;
                    return;
                }
                _ => {}
            }
        }

        if inner.termio.icanon() {
            match ch {
                0xff => {}
                ch if ch == inner.termio.veof() => {
                    if inner.line_len == 0 {
                        inner.buffer.push_back(0);
                    } else {
                        inner.buffer.push_back(0);
                        inner.line_len = 0;
                    }
                    self.wakeup_readers();
                    self.inner = inner;
                    return;
                }
                ch if ch == inner.termio.verase() || ch == CTRL!('H') => {
                    self.erase(&mut inner, true);
                    self.inner = inner;
                    return;
                }
                ch if ch == inner.termio.vkill() => {
                    if inner.termio.echok() {
                        while self.erase(&mut inner, false).is_some() {}
                        self.write_direct(&[b'\n']);
                        self.column = 0;
                    } else if inner.termio.echoke() && inner.termio.iexten() {
                        while self.erase(&mut inner, true).is_some() {}
                    } else {
                        while self.erase(&mut inner, false).is_some() {}
                    }
                    self.inner = inner;
                    return;
                }
                _ => {}
            }
        }

        match ch {
            b'\r' if inner.termio.igncr() => {}
            b'\r' if inner.termio.icrnl() => self.do_commit_char(&mut inner, b'\n'),
            b'\n' if inner.termio.inlcr() => self.do_commit_char(&mut inner, b'\r'),
            _ => self.do_commit_char(&mut inner, ch),
        }

        self.inner = inner;
    }

    pub fn read_available(&mut self, out: &mut [u8]) -> Option<usize> {
        if self.state & CARR_ON == 0 {
            return Some(0);
        }

        if self.inner.buffer.is_empty() {
            return None;
        }

        let length = if self.inner.termio.icanon() {
            self.inner
                .buffer
                .iter()
                .position(|&ch| ch == b'\n' || ch == 0)
                .map(|pos| pos + 1)
                .unwrap_or(0)
        } else {
            out.len().min(self.inner.buffer.len())
        };

        if length == 0 {
            return None;
        }

        let mut nread = 0;
        for _ in 0..length.min(out.len()) {
            let Some(ch) = self.inner.buffer.pop_front() else {
                break;
            };
            if ch == 0 {
                break;
            }
            out[nread] = ch;
            nread += 1;
            if self.inner.termio.icanon() && ch == b'\n' {
                break;
            }
        }

        Some(nread)
    }

    pub fn read_wait_channel(&self) -> usize {
        self as *const Tty as usize
    }

    pub fn flush(&mut self) {
        self.inner.buffer.clear();
        self.inner.line_len = 0;
        self.column = 0;
    }

    fn wakeup_readers(&self) {
        ProcessManager::get().wakeup_all(self.read_wait_channel());
    }

    fn erase_width(&mut self, width: usize) {
        for _ in 0..width {
            self.write_direct(&[CTRL!('H'), b' ', CTRL!('H')]);
            self.column = self.column.saturating_sub(1);
        }
    }

    fn write_output(&mut self, ch: u8) {
        match ch {
            b'\n' if self.inner.termio.oflag.contains(TermioOFlags::ONLCR) => {
                self.write_direct(&[b'\r', b'\n']);
                self.column = 0;
            }
            b'\t' => {
                let spaces = TAB_WIDTH - (self.column % TAB_WIDTH);
                for _ in 0..spaces {
                    self.write_direct(&[b' ']);
                    self.column += 1;
                }
            }
            CTRL!('H') => {
                self.write_direct(&[ch]);
                self.column = self.column.saturating_sub(1);
            }
            b'\r' => {
                self.write_direct(&[ch]);
                self.column = 0;
            }
            _ => {
                self.write_direct(&[ch]);
                self.column += 1;
            }
        }
    }

    fn write_direct(&mut self, data: &[u8]) {
        serial::serial_write_bytes(data.iter().copied());
    }
}

static CONSOLE_TTY: LazyLock<Spin<Tty>> = LazyLock::new(|| Spin::new(Tty::new()));

pub fn console_tty() -> SpinGuard<'static, Tty, IrqContext> {
    CONSOLE_TTY.lock_ctx::<IrqContext>()
}

pub fn tty_input_byte(ch: u8) {
    console_tty().input(ch);
}

pub fn tty_flush() {
    console_tty().flush();
}
