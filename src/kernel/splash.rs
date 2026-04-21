use core::mem::size_of;

use crate::{
    mm::{alloc_page, free_page},
    println,
    vesa::{vesa_clear, vesa_put_pixel},
};

use super::syscall::{_lib_open, _lib_read, _lib_seek, _lib_sleep};

const KERNEL_SPACE_START_ADDRESS: usize = 0xc0000000;
const SPLASH_BMP: *const u8 = b"/etc/v6pp_splash.bmp\0".as_ptr();
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

#[no_mangle]
pub extern "C" fn splash() -> i32 {
    vesa_clear(0);
    draw_img(SPLASH_BMP);
    _lib_sleep(1);
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

    let image_size = image_size as usize;
    let phys_buf = unsafe { alloc_page(image_size, false) };
    if phys_buf == 0 {
        println!("splash: failed to alloc memory for splash buffer!");
        return -1;
    }

    let result = {
        let img_data = (phys_buf + KERNEL_SPACE_START_ADDRESS) as *mut u8;

        _lib_seek(file, image_offset as u32, 0);
        if _lib_read(file, img_data, image_size as i32) != image_size as i32 {
            -1
        } else {
            draw_bgra_image(img_data, width, height);
            0
        }
    };

    free_page(phys_buf, image_size, false);

    result
}

fn draw_bgra_image(img_data: *const u8, width: i32, height: i32) {
    for h in (0..height).rev() {
        for w in 0..width {
            let pixel_offset = (w + h * width) as usize * size_of::<i32>();
            let pixel = unsafe { img_data.add(pixel_offset).cast::<i32>().read_unaligned() };
            vesa_put_pixel(w, height - h, pixel);
        }
    }
}
