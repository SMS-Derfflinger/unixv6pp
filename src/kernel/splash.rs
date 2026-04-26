use core::mem::size_of;

use crate::{
    interrupt::kernel_delay_seconds,
    mm::{phys_to_virt, KernelPages},
    println,
    vesa::{vesa_clear, vesa_put_pixel},
};

use super::syscall::{_lib_open, _lib_read, _lib_seek};

const SPLASH_BMP: *const u8 = b"/v6pp_splash.bmp\0".as_ptr();
const OPEN_READ_MODE: u32 = 0o111;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct BmpFileHeader {
    magic: [u8; 2],
    file_size: i32,
    reserved0: i16,
    reserved1: i16,
    offset: i32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct BmpInfoHeader {
    header_size: i32,
    width: i32,
    height: i32,
    number_of_color_planes: i16,
    bits_per_pixel: i16,
    compression_method: i32,
    image_size: i32,
    horizontal_resolution: i32,
    vertical_resolution: i32,
    number_of_colors_in_color_palette: i32,
    number_of_important_colors: i32,
}

pub fn splash() -> i32 {
    vesa_clear(0);
    draw_img(SPLASH_BMP);
    kernel_delay_seconds(1);
    vesa_clear(0);

    0
}

fn draw_img(file_path: *const u8) -> i32 {
    let file = _lib_open(file_path, OPEN_READ_MODE);
    if file == -1 {
        return -1;
    }

    _lib_seek(file, 0, 0);

    let mut file_header = BmpFileHeader {
        magic: [0; 2],
        file_size: 0,
        reserved0: 0,
        reserved1: 0,
        offset: 0,
    };

    if _lib_read(
        file,
        (&raw mut file_header).cast::<u8>(),
        size_of::<BmpFileHeader>() as i32,
    ) != size_of::<BmpFileHeader>() as i32
    {
        return -1;
    }

    if file_header.magic != [b'B', b'M'] {
        println!("splash: bad magic!");
        return -1;
    }

    let mut info_header = BmpInfoHeader {
        header_size: 0,
        width: 0,
        height: 0,
        number_of_color_planes: 0,
        bits_per_pixel: 0,
        compression_method: 0,
        image_size: 0,
        horizontal_resolution: 0,
        vertical_resolution: 0,
        number_of_colors_in_color_palette: 0,
        number_of_important_colors: 0,
    };

    if _lib_read(
        file,
        (&raw mut info_header).cast::<u8>(),
        size_of::<BmpInfoHeader>() as i32,
    ) != size_of::<BmpInfoHeader>() as i32
    {
        return -1;
    }

    let width = info_header.width;
    let height = info_header.height;
    let image_size = info_header.image_size;
    let image_offset = file_header.offset;

    if width <= 0 || height <= 0 || image_size <= 0 || image_offset < 0 {
        return -1;
    }

    let row_size = width as usize * size_of::<i32>();
    if row_size > image_size as usize {
        return -1;
    }

    let pages = KernelPages::alloc_bytes(row_size);
    let row_buf = phys_to_virt(pages.phys());

    _lib_seek(file, image_offset as u32, 0);
    for row in 0..height {
        if _lib_read(file, row_buf, row_size as i32) != row_size as i32 {
            return -1;
        }

        draw_bgra_row(row_buf, width, height - row);
    }

    0
}

fn draw_bgra_row(row_data: *const u8, width: i32, y: i32) {
    for w in 0..width {
        let pixel_offset = w as usize * size_of::<i32>();
        let pixel = unsafe { row_data.add(pixel_offset).cast::<i32>().read_unaligned() };
        vesa_put_pixel(w, y, pixel);
    }
}
