pub mod asm;
mod chip;

use core::mem::size_of;

use crate::sync::SuperCell;

const DESCRIPTOR_COUNT: usize = 256;
const KERNEL_DATA_SEGMENT_SELECTOR: u32 = 0x10;
const USER_CODE_SEGMENT_SELECTOR: u32 = 0x18 | 0x3;
const USER_DATA_SEGMENT_SELECTOR: u32 = 0x20 | 0x3;
const PAGE_DIRECTORY_BASE_ADDRESS: u32 = 0x200000;
const KERNEL_SPACE_START_ADDRESS: u32 = 0xc0000000;
const KERNEL_SPACE_SIZE: u32 = 0x400000;
const TASK_STATE_SEGMENT_INDEX: usize = 5;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    flags_limit_high: u8,
    base_high: u8,
}

impl SegmentDescriptor {
    const fn empty() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            flags_limit_high: 0,
            base_high: 0,
        }
    }

    fn new(
        base_address: u32,
        segment_limit: u32,
        descriptor_type: u8,
        system: u8,
        dpl: u8,
        present: u8,
        default_operation_size: u8,
        granularity: u8,
        available: u8,
    ) -> Self {
        let mut descriptor = Self::empty();
        descriptor.set_segment_limit(segment_limit);
        descriptor.set_base_address(base_address);
        descriptor.access = (descriptor_type & 0x0f)
            | ((system & 0x01) << 4)
            | ((dpl & 0x03) << 5)
            | ((present & 0x01) << 7);
        descriptor.flags_limit_high = (descriptor.flags_limit_high & 0x0f)
            | ((available & 0x01) << 4)
            | ((default_operation_size & 0x01) << 6)
            | ((granularity & 0x01) << 7);
        descriptor
    }

    fn set_base_address(&mut self, base_address: u32) {
        self.base_low = (base_address & 0xffff) as u16;
        self.base_mid = ((base_address >> 16) & 0xff) as u8;
        self.base_high = ((base_address >> 24) & 0xff) as u8;
    }

    fn set_segment_limit(&mut self, segment_limit: u32) {
        self.limit_low = (segment_limit & 0xffff) as u16;
        self.flags_limit_high =
            (self.flags_limit_high & 0xf0) | ((segment_limit >> 16) & 0x0f) as u8;
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GateDescriptor {
    offset_low: u16,
    selector: u16,
    reserved: u8,
    access: u8,
    offset_high: u16,
}

impl GateDescriptor {
    const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            reserved: 0,
            access: 0,
            offset_high: 0,
        }
    }

    fn new(handler: u32, gate_type: u8) -> Self {
        Self {
            offset_low: (handler & 0xffff) as u16,
            selector: 0x0008,
            reserved: 0,
            access: gate_type | (0x03 << 5) | 0x80,
            offset_high: (handler >> 16) as u16,
        }
    }
}

#[repr(C, packed)]
pub struct DescriptorTableRegister {
    limit: u16,
    base: u32,
}

impl DescriptorTableRegister {
    fn new<T>(table: *const T, bytes: usize) -> Self {
        Self {
            limit: (bytes - 1) as u16,
            base: table as u32,
        }
    }
}

#[repr(C, packed)]
pub struct Gdt {
    descriptors: [SegmentDescriptor; DESCRIPTOR_COUNT],
}

impl Gdt {
    const fn empty() -> Self {
        Self {
            descriptors: [SegmentDescriptor::empty(); DESCRIPTOR_COUNT],
        }
    }

    fn init(&mut self) {
        self.descriptors = [SegmentDescriptor::empty(); DESCRIPTOR_COUNT];
        self.set_descriptor_values(1, 0, 0xfffff, 0x0a, 1, 0, 1, 1, 1, 0);
        self.set_descriptor_values(2, 0, 0xfffff, 0x02, 1, 0, 1, 1, 1, 0);
        self.set_descriptor_values(3, 0, 0xfffff, 0x0a, 1, 3, 1, 1, 1, 0);
        self.set_descriptor_values(4, 0, 0xfffff, 0x02, 1, 3, 1, 1, 1, 0);
        self.init_tss_descriptor();
    }

    fn set_descriptor_values(
        &mut self,
        index: usize,
        base_address: u32,
        segment_limit: u32,
        descriptor_type: u8,
        system: u8,
        dpl: u8,
        present: u8,
        default_operation_size: u8,
        granularity: u8,
        available: u8,
    ) {
        self.descriptors[index] = SegmentDescriptor::new(
            base_address,
            segment_limit,
            descriptor_type,
            system,
            dpl,
            present,
            default_operation_size,
            granularity,
            available,
        );
    }

    fn init_tss_descriptor(&mut self) {
        self.descriptors[TASK_STATE_SEGMENT_INDEX] = SegmentDescriptor::new(
            task_state_segment_ptr() as u32,
            size_of::<TaskStateSegment>() as u32 - 1,
            0x09,
            0,
            0,
            1,
            0,
            1,
            1,
        );
    }
}

#[repr(C, packed)]
pub struct Idt {
    descriptors: [GateDescriptor; DESCRIPTOR_COUNT],
}

impl Idt {
    const fn empty() -> Self {
        Self {
            descriptors: [GateDescriptor::empty(); DESCRIPTOR_COUNT],
        }
    }

    fn init(&mut self) {
        self.descriptors = [GateDescriptor::empty(); DESCRIPTOR_COUNT];
    }

    fn set_gate(&mut self, number: usize, handler: u32, gate_type: u8) {
        self.descriptors[number] = GateDescriptor::new(handler, gate_type);
    }
}

#[repr(C, packed)]
pub struct TaskStateSegment {
    previous_task_link: u32,
    esp0: u32,
    ss0: u32,
    esp1: u32,
    ss1: u32,
    esp2: u32,
    ss2: u32,
    cr3: u32,
    eip: u32,
    eflags: u32,
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    esp: u32,
    ebp: u32,
    esi: u32,
    edi: u32,
    es: u32,
    cs: u32,
    ss: u32,
    ds: u32,
    fs: u32,
    gs: u32,
    ldt: u32,
    trap_and_reserved: u16,
    io_map_base: u16,
}

impl TaskStateSegment {
    const fn empty() -> Self {
        Self {
            previous_task_link: 0,
            esp0: 0,
            ss0: 0,
            esp1: 0,
            ss1: 0,
            esp2: 0,
            ss2: 0,
            cr3: 0,
            eip: 0,
            eflags: 0,
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0,
            ebp: 0,
            esi: 0,
            edi: 0,
            es: 0,
            cs: 0,
            ss: 0,
            ds: 0,
            fs: 0,
            gs: 0,
            ldt: 0,
            trap_and_reserved: 0,
            io_map_base: 0,
        }
    }

    fn init(&mut self) {
        *self = Self::empty();
        self.cr3 = PAGE_DIRECTORY_BASE_ADDRESS;
        self.cs = USER_CODE_SEGMENT_SELECTOR;
        self.ds = USER_DATA_SEGMENT_SELECTOR;
        self.ss = USER_DATA_SEGMENT_SELECTOR;
        self.es = USER_DATA_SEGMENT_SELECTOR;
        self.fs = USER_DATA_SEGMENT_SELECTOR;
        self.gs = USER_DATA_SEGMENT_SELECTOR;
        self.ebp = KERNEL_SPACE_START_ADDRESS + KERNEL_SPACE_SIZE;
        self.esp = KERNEL_SPACE_START_ADDRESS + KERNEL_SPACE_SIZE;
        self.eip = KERNEL_SPACE_START_ADDRESS;
        self.eflags = 0x200;
        self.ss0 = KERNEL_DATA_SEGMENT_SELECTOR;
        self.esp0 = KERNEL_SPACE_START_ADDRESS + KERNEL_SPACE_SIZE;
    }
}

static GDT: SuperCell<Gdt> = SuperCell::new(Gdt::empty());
static IDT: SuperCell<Idt> = SuperCell::new(Idt::empty());
static TSS: SuperCell<TaskStateSegment> = SuperCell::new(TaskStateSegment::empty());

#[no_mangle]
pub extern "C" fn _gdt_init() {
    GDT.with_mut(|gdt| gdt.init());
}

#[no_mangle]
pub extern "C" fn _gdt_form_gdtr(gdtr: *mut DescriptorTableRegister) {
    GDT.with(|gdt| unsafe {
        *gdtr = DescriptorTableRegister::new(gdt as *const Gdt, size_of::<Gdt>());
    });
}

#[no_mangle]
pub extern "C" fn _idt_init() {
    IDT.with_mut(|idt| idt.init());
}

#[no_mangle]
pub extern "C" fn _idt_set_interrupt_gate(number: i32, handler: u32) {
    IDT.with_mut(|idt| idt.set_gate(number as usize, handler, 0x0e));
}

#[no_mangle]
pub extern "C" fn _idt_set_trap_gate(number: i32, handler: u32) {
    IDT.with_mut(|idt| idt.set_gate(number as usize, handler, 0x0f));
}

#[no_mangle]
pub extern "C" fn _idt_form_idtr(idtr: *mut DescriptorTableRegister) {
    IDT.with(|idt| unsafe {
        *idtr = DescriptorTableRegister::new(idt as *const Idt, size_of::<Idt>());
    });
}

#[no_mangle]
pub extern "C" fn _idt_default_interrupt_handler() {
    panic!("Default Interrupt Handler!");
}

#[no_mangle]
pub extern "C" fn _idt_default_exception_handler() {
    panic!("Default Exception Handler!");
}

#[no_mangle]
pub extern "C" fn _task_state_segment_init() {
    TSS.with_mut(|tss| tss.init());
    GDT.with_mut(|gdt| gdt.init_tss_descriptor());
}

#[no_mangle]
pub extern "C" fn _task_state_segment_ptr() -> *mut TaskStateSegment {
    task_state_segment_ptr()
}

fn task_state_segment_ptr() -> *mut TaskStateSegment {
    TSS.get_mut() as *mut TaskStateSegment
}
