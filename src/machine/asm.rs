use riscv::register::{sscratch, sstatus, stvec};

pub fn disable_interrupts() {
    unsafe { sstatus::clear_sie() }
}

pub fn enable_interrupts() {
    unsafe { sstatus::set_sie() }
}

pub fn write_stvec(addr: usize) {
    let mut value = stvec::Stvec::from_bits(0);
    value.set_trap_mode(stvec::TrapMode::Direct);
    value.set_address(addr);
    unsafe { stvec::write(value) }
}

pub fn write_sscratch(addr: usize) {
    unsafe { sscratch::write(addr) }
}
