use eonix_sync_base::LazyLock;

use core::{arch::asm, ptr::write_volatile};

use crate::{sync::SuperCell, vesa};

pub mod keyboard;

const TTY_BUF_SIZE: usize = 512;
const CANBSIZ: usize = 256;

const CERASE: u8 = b'\x08';
const CEOT: u8 = 0x04;
const CKILL: u8 = 0x15;

const TTHIWAT: usize = 512;

const ECHO: u32 = 0x08;
const CRMOD: u32 = 0x10;
const RAW: u32 = 0x20;
const LCASE: u32 = 0x04;

const ISOPEN: u32 = 0x01;
const CARR_ON: u32 = 0x02;

const TTIPRI: i32 = 10;

unsafe extern "C" {
    fn cpp_process_sleep(chan: usize, pri: i32);
    fn cpp_process_wakeup_all(chan: usize);
}

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

const VIDEO_MEMORY: usize = 0xc00b_8000;
const VIDEO_ADDR_PORT: u16 = 0x3d4;
const VIDEO_DATA_PORT: u16 = 0x3d5;
const CRT_COLUMNS: usize = 80;
const CRT_ROWS: usize = 15;
const CRT_COLOR: u16 = 0x0f00;

struct TextModeConsole {
    cursor_x: usize,
    cursor_y: usize,
    input_begin: usize,
}

impl TextModeConsole {
    const fn new() -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            input_begin: 0,
        }
    }

    fn put_char(&mut self, ch: u8) {
        match ch {
            b'\n' => self.next_line(),
            CKILL => {}
            CERASE => self.backspace(),
            b'\t' => self.tab(),
            _ => self.write_visible(ch),
        }
    }

    fn mark_input_begin(&mut self) {
        self.input_begin = self.cursor_pos();
    }

    fn next_line(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;

        if self.cursor_y >= CRT_ROWS {
            self.cursor_y = 0;
            self.clear_screen();
        }

        self.move_cursor();
        self.mark_input_begin();
    }

    fn backspace(&mut self) {
        if self.cursor_pos() == self.input_begin {
            return;
        }

        if self.cursor_x == 0 {
            if self.cursor_y == 0 {
                return;
            }

            self.cursor_y -= 1;
            self.cursor_x = CRT_COLUMNS - 1;
        } else {
            self.cursor_x -= 1;
        }

        self.write_at(self.cursor_x, self.cursor_y, b' ');
        self.move_cursor();
    }

    fn tab(&mut self) {
        let next_tab = (self.cursor_x & !0x7) + 8;
        if next_tab >= CRT_COLUMNS {
            self.next_line();
        } else {
            while self.cursor_x < next_tab {
                self.write_visible(b' ');
            }
        }
    }

    fn write_visible(&mut self, ch: u8) {
        self.write_at(self.cursor_x, self.cursor_y, ch);
        self.cursor_x += 1;

        if self.cursor_x >= CRT_COLUMNS {
            self.next_line();
        } else {
            self.move_cursor();
        }
    }

    fn clear_screen(&mut self) {
        for row in 0..CRT_ROWS {
            for col in 0..CRT_COLUMNS {
                self.write_at(col, row, b' ');
            }
        }
    }

    fn write_at(&self, col: usize, row: usize, ch: u8) {
        let cell = (VIDEO_MEMORY as *mut u16).wrapping_add(row * CRT_COLUMNS + col);
        unsafe {
            write_volatile(cell, CRT_COLOR | ch as u16);
        }
    }

    fn move_cursor(&self) {
        let pos = self.cursor_pos() as u16;
        unsafe {
            out_byte(VIDEO_ADDR_PORT, 14);
            out_byte(VIDEO_DATA_PORT, (pos >> 8) as u8);
            out_byte(VIDEO_ADDR_PORT, 15);
            out_byte(VIDEO_DATA_PORT, (pos & 0xff) as u8);
        }
    }

    fn cursor_pos(&self) -> usize {
        self.cursor_y * CRT_COLUMNS + self.cursor_x
    }
}

struct TextModeOutput;

impl TextModeOutput {
    fn put_char(ch: u8) {
        if vesa::write_output_byte(ch) {
            return;
        }

        TEXT_CONSOLE.with_mut(|console| console.put_char(ch));
    }

    fn mark_input_begin() {
        if vesa::mark_input_begin() {
            return;
        }

        TEXT_CONSOLE.with_mut(TextModeConsole::mark_input_begin);
    }
}

unsafe fn out_byte(port: u16, data: u8) {
    asm!(
        "outb %al, %dx",
        in("dx") port,
        in("al") data,
        options(nomem, nostack, att_syntax)
    );
}

static TEXT_CONSOLE: LazyLock<SuperCell<TextModeConsole>> =
    LazyLock::new(|| SuperCell::new(TextModeConsole::new()));

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
            self.flags = ECHO;
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
        TextModeOutput::mark_input_begin();
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
            unsafe {
                cpp_process_wakeup_all(self.read_wait_channel());
            }
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
            TextModeOutput::put_char(ch);
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

pub fn sleep_on_input_channel(chan: usize) {
    unsafe {
        cpp_process_sleep(chan, TTIPRI);
    }
}

#[no_mangle]
pub extern "C" fn tty_input_byte(ch: u8) {
    console_tty().with_mut(|tty| tty.input(ch));
}

#[no_mangle]
pub extern "C" fn tty_flush() {
    console_tty().with_mut(Tty::flush);
}

#[no_mangle]
pub extern "C" fn clear_screan() {
    TEXT_CONSOLE.with_mut(TextModeConsole::clear_screen);
}
