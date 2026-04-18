mod chip;
pub mod asm;

#[no_mangle]
pub unsafe extern "C" fn _segment_descriptor_set_base_address(
    descriptor: *mut u8,
    base_address: u32,
) {
    set_u16(descriptor.add(2), (base_address & 0xffff) as u16);
    *descriptor.add(4) = ((base_address >> 16) & 0xff) as u8;
    *descriptor.add(7) = ((base_address >> 24) & 0xff) as u8;
}

#[no_mangle]
pub unsafe extern "C" fn _segment_descriptor_set_segment_limit(
    descriptor: *mut u8,
    segment_limit: u32,
) {
    set_u16(descriptor, (segment_limit & 0xffff) as u16);
    let flags = *descriptor.add(6) & 0xf0;
    *descriptor.add(6) = flags | ((segment_limit >> 16) & 0x0f) as u8;
}

#[no_mangle]
pub unsafe extern "C" fn _gdt_form_gdtr(gdt: *const u8, gdtr: *mut u8) {
    set_u16(gdtr, 2048 - 1);
    set_u32(gdtr.add(2), gdt as u32);
}

#[no_mangle]
pub unsafe extern "C" fn _idt_set_interrupt_gate(idt: *mut u8, number: i32, handler: u32) {
    idt_set_gate(idt, number, handler, 0x0e);
}

#[no_mangle]
pub unsafe extern "C" fn _idt_set_trap_gate(idt: *mut u8, number: i32, handler: u32) {
    idt_set_gate(idt, number, handler, 0x0f);
}

#[no_mangle]
pub unsafe extern "C" fn _idt_form_idtr(idt: *const u8, idtr: *mut u8) {
    set_u16(idtr, 2048 - 1);
    set_u32(idtr.add(2), idt as u32);
}

#[no_mangle]
pub unsafe extern "C" fn _tss_descriptor_set_base_address(
    descriptor: *mut u8,
    base_address: u32,
) {
    _segment_descriptor_set_base_address(descriptor, base_address);
}

#[no_mangle]
pub unsafe extern "C" fn _tss_descriptor_set_segment_limit(
    descriptor: *mut u8,
    segment_limit: u32,
) {
    _segment_descriptor_set_segment_limit(descriptor, segment_limit);
}

unsafe fn idt_set_gate(idt: *mut u8, number: i32, handler: u32, gate_type: u8) {
    let descriptor = idt.add(number as usize * 8);

    set_u16(descriptor, (handler & 0xffff) as u16);
    set_u16(descriptor.add(2), 0x0008);
    *descriptor.add(4) = 0;
    *descriptor.add(5) = gate_type | (0x03 << 5) | 0x80;
    set_u16(descriptor.add(6), (handler >> 16) as u16);
}

unsafe fn set_u16(dst: *mut u8, value: u16) {
    *dst = value as u8;
    *dst.add(1) = (value >> 8) as u8;
}

unsafe fn set_u32(dst: *mut u8, value: u32) {
    *dst = value as u8;
    *dst.add(1) = (value >> 8) as u8;
    *dst.add(2) = (value >> 16) as u8;
    *dst.add(3) = (value >> 24) as u8;
}
