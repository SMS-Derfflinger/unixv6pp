use core::arch::asm;

use eonix_sync_base::LazyLock;

use crate::constants::Signal;
use crate::proc::ProcessManager;
use crate::sync::SuperCell;
use crate::tty::{tty_flush, tty_input_byte};

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const DATA_BUFFER_BUSY: u8 = 0x01;

const SCAN_ALT: u8 = 0x38;
const SCAN_CTRL: u8 = 0x1d;
const SCAN_LSHIFT: u8 = 0x2a;
const SCAN_RSHIFT: u8 = 0x36;
const SCAN_NUMLOCK: u8 = 0x45;
const SCAN_CAPSLOCK: u8 = 0x3a;
const SCAN_SCRLOCK: u8 = 0x46;

const M_LCTRL: u32 = 0x01;
const M_RCTRL: u32 = 0x02;
const M_LALT: u32 = 0x04;
const M_RALT: u32 = 0x08;
const M_LSHIFT: u32 = 0x10;
const M_RSHIFT: u32 = 0x20;
const M_NUMLOCK: u32 = 0x40;
const M_CAPSLOCK: u32 = 0x80;
const M_SCRLOCK: u32 = 0x100;
const M_DOWN_NUMLOCK: u32 = 0x200;
const M_DOWN_CAPSLOCK: u32 = 0x400;
const M_DOWN_SCRLOCK: u32 = 0x800;

const KEYMAP: [u8; 0x58] = [
    0, 0x1b, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 0x08, 0x09,
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', 0, b'a', b's',
    b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v',
    b'b', b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    b'7', b'8', b'9', b'-', b'4', b'5', b'6', b'+', b'1', b'2', b'3', b'0', b'.', 0, 0, 0, 0,
];

const SHIFT_KEYMAP: [u8; 0x58] = [
    0, 0x1b, b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'_', b'+', 0x08, 0x09,
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'{', b'}', b'\n', 0, b'a', b's',
    b'd', b'f', b'g', b'h', b'j', b'k', b'l', b':', b'"', b'~', 0, b'|', b'z', b'x', b'c', b'v',
    b'b', b'n', b'm', b'<', b'>', b'?', 0, b'*', 0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, b'-', 0, 0, 0, b'+', 0, 0, 0, 0, 0x7f, 0, 0, 0, 0,
];

struct KeyboardState {
    mode: u32,
    pre_state: i32,
}

impl KeyboardState {
    const fn new() -> Self {
        Self {
            mode: 0,
            pre_state: 0,
        }
    }

    fn handle_interrupt(&mut self) {
        let mut status = unsafe { in_byte(STATUS_PORT) };
        let mut limit = 10;

        while status & DATA_BUFFER_BUSY != 0 && limit > 0 {
            limit -= 1;
            let scancode = unsafe { in_byte(DATA_PORT) };

            match self.pre_state {
                0 => {
                    if scancode == 0xe0 || scancode == 0xe1 {
                        self.pre_state = scancode as i32;
                    } else {
                        self.handle_scan_code(scancode, 0);
                    }
                }
                0xe0 => {
                    self.pre_state = 0;
                    self.handle_scan_code(scancode, 0xe0);
                }
                0xe1 if scancode == 0x1d || scancode == 0x9d => {
                    self.pre_state = 0x100;
                }
                0x100 if scancode == 0x45 => {
                    self.pre_state = 0;
                    self.handle_scan_code(scancode, 0xe1);
                }
                _ => self.pre_state = 0,
            }

            status = unsafe { in_byte(STATUS_PORT) };
        }
    }

    fn handle_scan_code(&mut self, scan_code: u8, expand: i32) {
        let ch = match scan_code {
            SCAN_ALT => {
                if expand == 0xe0 {
                    self.mode |= M_RALT;
                } else {
                    self.mode |= M_LALT;
                }
                0
            }
            SCAN_CTRL => {
                if expand == 0xe0 {
                    self.mode |= M_RCTRL;
                } else {
                    self.mode |= M_LCTRL;
                }
                0
            }
            SCAN_LSHIFT => {
                self.mode |= M_LSHIFT;
                0
            }
            SCAN_RSHIFT => {
                self.mode |= M_RSHIFT;
                0
            }
            code if code == SCAN_ALT + 0x80 => {
                if expand == 0xe0 {
                    self.mode &= !M_RALT;
                } else {
                    self.mode &= !M_LALT;
                }
                0
            }
            code if code == SCAN_CTRL + 0x80 => {
                if expand == 0xe0 {
                    self.mode &= !M_RCTRL;
                } else {
                    self.mode &= !M_LCTRL;
                }
                0
            }
            code if code == SCAN_LSHIFT + 0x80 => {
                self.mode &= !M_LSHIFT;
                0
            }
            code if code == SCAN_RSHIFT + 0x80 => {
                self.mode &= !M_RSHIFT;
                0
            }
            SCAN_NUMLOCK => {
                if self.mode & M_DOWN_NUMLOCK == 0 {
                    self.mode ^= M_NUMLOCK;
                    self.mode |= M_DOWN_NUMLOCK;
                }
                0
            }
            SCAN_CAPSLOCK => {
                if self.mode & M_DOWN_CAPSLOCK == 0 {
                    self.mode ^= M_CAPSLOCK;
                    self.mode |= M_DOWN_CAPSLOCK;
                }
                0
            }
            SCAN_SCRLOCK => {
                if self.mode & M_DOWN_SCRLOCK == 0 {
                    self.mode ^= M_SCRLOCK;
                    self.mode |= M_DOWN_SCRLOCK;
                }
                0
            }
            code if code == SCAN_NUMLOCK + 0x80 => {
                self.mode &= !M_DOWN_NUMLOCK;
                0
            }
            code if code == SCAN_CAPSLOCK + 0x80 => {
                self.mode &= !M_DOWN_CAPSLOCK;
                0
            }
            code if code == SCAN_SCRLOCK + 0x80 => {
                self.mode &= !M_DOWN_SCRLOCK;
                0
            }
            _ => self.translate(scan_code, expand),
        };

        if ch != 0 {
            tty_input_byte(ch);
        }
    }

    fn translate(&mut self, scan_code: u8, expand: i32) -> u8 {
        if expand == 0xe1 {
            return 0x05;
        }

        if scan_code < 0x45 {
            let mut ch = if self.shift_down() {
                SHIFT_KEYMAP[scan_code as usize]
            } else {
                KEYMAP[scan_code as usize]
            };

            if ch.is_ascii_lowercase() {
                let reverse = (self.mode & M_CAPSLOCK != 0) ^ self.shift_down();

                if self.ctrl_down() {
                    if ch == b'c' {
                        tty_flush();
                        ProcessManager::get().raise(core::ptr::null_mut(), Signal::SIGINT);
                        ch = 0;
                    } else {
                        ch = ch - b'a' + 1;
                    }
                } else if reverse {
                    ch = ch.to_ascii_uppercase();
                }
            }

            return ch;
        }

        if scan_code < 0x58 {
            let reverse = (self.mode & M_NUMLOCK != 0) ^ self.shift_down();

            return if expand == 0xe0 {
                SHIFT_KEYMAP[scan_code as usize]
            } else if reverse {
                KEYMAP[scan_code as usize]
            } else {
                SHIFT_KEYMAP[scan_code as usize]
            };
        }

        0
    }

    fn ctrl_down(&self) -> bool {
        self.mode & (M_LCTRL | M_RCTRL) != 0
    }

    fn shift_down(&self) -> bool {
        self.mode & (M_LSHIFT | M_RSHIFT) != 0
    }
}

static KEYBOARD: LazyLock<SuperCell<KeyboardState>> =
    LazyLock::new(|| SuperCell::new(KeyboardState::new()));

#[no_mangle]
pub extern "C" fn keyboard_handle_interrupt() {
    KEYBOARD.with_mut(KeyboardState::handle_interrupt);
}

unsafe fn in_byte(port: u16) -> u8 {
    let data: u8;
    asm!(
        "inb %dx, %al",
        out("al") data,
        in("dx") port,
        options(nomem, nostack, att_syntax)
    );
    data
}
