pub const MAX_DEVICE_NUM: usize = 10;
pub const NODEV: i16 = -1;
pub const ROOTDEV: i16 = 0;
pub const TTYDEV: i16 = 0;

pub const fn major(dev: i16) -> i16 {
    dev >> 8
}

pub const fn minor(dev: i16) -> i16 {
    dev & 0x00ff
}

pub const fn set_major(dev: i16, value: i16) -> i16 {
    (dev & 0x00ff) | (value << 8)
}

pub const fn set_minor(dev: i16, value: i16) -> i16 {
    (dev & 0xff00u16 as i16) | (value & 0x00ff)
}
