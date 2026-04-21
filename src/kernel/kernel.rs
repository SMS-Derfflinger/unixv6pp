use crate::{println, serial::init_serial};

unsafe extern "C" {
    fn cpp_swapper_manager_initialize() -> i32;
    fn cpp_process_manager_initialize();
    fn buffer_manager_initialize();
}

pub fn rust_kernel_initialize() {
    init_serial();
    init_memory();
    init_process();
    init_buffer();
}

fn init_memory() {
    println!("Initilize Memory...");
    println!("Ok.");

    println!("Initialize Swapper...");
    unsafe {
        cpp_swapper_manager_initialize();
    }
    println!("Ok.");
}

fn init_process() {
    println!("Initilize Process...");
    unsafe {
        cpp_process_manager_initialize();
    }
    println!("Ok.");
}

fn init_buffer() {
    println!("Initialize Buffer...");
    unsafe {
        buffer_manager_initialize();
    }
    println!("OK.");

    println!("Initialize Device Manager...");
    println!("OK.");
}
