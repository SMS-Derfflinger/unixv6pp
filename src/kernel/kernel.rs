use crate::{dev::buffer_manager::buffer_manager_initialize, println, serial::init_serial};

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
    println!("Ok.");
}

fn init_process() {
    println!("Initilize Process...");
    println!("Ok.");
}

fn init_buffer() {
    println!("Initialize Buffer...");
    buffer_manager_initialize();
    println!("OK.");

    println!("Initialize Device Manager...");
    println!("OK.");
}
