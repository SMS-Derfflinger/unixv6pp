use crate::serial::{serial_write, serial_write_bytes};
use crate::sync::SuperCell;

const SCREEN_ROWS: u32 = 25;

static TRACE_ON: SuperCell<bool> = SuperCell::new(true);
static ROWS: SuperCell<u32> = SuperCell::new(10);
static ROW: SuperCell<u32> = SuperCell::new(10);
static COLUMN: SuperCell<u32> = SuperCell::new(0);

#[no_mangle]
pub extern "C" fn _diagnose_trace_on() {
    TRACE_ON.with_mut(|trace_on| *trace_on = true);
}

#[no_mangle]
pub extern "C" fn _diagnose_trace_off() {
    TRACE_ON.with_mut(|trace_on| *trace_on = false);
}

#[no_mangle]
pub extern "C" fn _diagnose_is_trace_on() -> bool {
    TRACE_ON.with(|trace_on| *trace_on)
}

#[no_mangle]
pub extern "C" fn _diagnose_write_cstr(message: *const u8) {
    if !_diagnose_is_trace_on() {
        return;
    }

    write_c_string(message);
}

#[no_mangle]
pub extern "C" fn _diagnose_clear_screen() {
    let rows = diagnose_rows();
    ROW.with_mut(|row| *row = SCREEN_ROWS - rows);
    COLUMN.with_mut(|column| *column = 0);
}

pub fn diagnose_rows() -> u32 {
    ROWS.with(|rows| *rows)
}

pub fn diagnose_enable_rows(rows: u32) {
    ROWS.with_mut(|diagnose_rows| *diagnose_rows = rows);
    ROW.with_mut(|row| *row = SCREEN_ROWS - rows);
    COLUMN.with_mut(|column| *column = 0);
}

pub fn diagnose_disable_rows() {
    _diagnose_clear_screen();
    ROWS.with_mut(|rows| *rows = 0);
    ROW.with_mut(|row| *row = SCREEN_ROWS);
    COLUMN.with_mut(|column| *column = 0);
}

fn write_c_string(message: *const u8) {
    if message.is_null() {
        serial_write("(null)");
        return;
    }

    let mut cursor = message;
    let bytes = core::iter::from_fn(move || {
        let byte = unsafe { cursor.read() };
        if byte == 0 {
            None
        } else {
            cursor = unsafe { cursor.add(1) };
            Some(byte)
        }
    });

    serial_write_bytes(bytes);
}
