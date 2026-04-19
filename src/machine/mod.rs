pub mod asm;
mod chip;
mod page_table;

use core::arch::asm;
use core::mem::size_of;

use crate::sync::SuperCell;

const DESCRIPTOR_COUNT: usize = 256;
const KERNEL_DATA_SEGMENT_SELECTOR: u32 = 0x10;
const USER_CODE_SEGMENT_SELECTOR: u32 = 0x18 | 0x3;
const USER_DATA_SEGMENT_SELECTOR: u32 = 0x20 | 0x3;
const TASK_STATE_SEGMENT_SELECTOR: u16 = 0x28;
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

    fn init_gates(&mut self, handlers: &MachineIdtHandlers) {
        self.init();

        for number in 0..DESCRIPTOR_COUNT {
            if number < 32 {
                self.set_gate(
                    number,
                    idt_default_exception_handler as *const () as u32,
                    0x0f,
                );
            } else {
                self.set_gate(
                    number,
                    idt_default_interrupt_handler as *const () as u32,
                    0x0e,
                );
            }
        }

        self.set_gate(0, handlers.divide_error, 0x0f);
        self.set_gate(1, handlers.debug, 0x0f);
        self.set_gate(2, handlers.nmi, 0x0f);
        self.set_gate(3, handlers.breakpoint, 0x0f);
        self.set_gate(4, handlers.overflow, 0x0f);
        self.set_gate(5, handlers.bound, 0x0f);
        self.set_gate(6, handlers.invalid_opcode, 0x0f);
        self.set_gate(7, handlers.device_not_available, 0x0f);
        self.set_gate(8, handlers.double_fault, 0x0f);
        self.set_gate(9, handlers.coprocessor_segment_overrun, 0x0f);
        self.set_gate(10, handlers.invalid_tss, 0x0f);
        self.set_gate(11, handlers.segment_not_present, 0x0f);
        self.set_gate(12, handlers.stack_segment_error, 0x0f);
        self.set_gate(13, handlers.general_protection, 0x0f);
        self.set_gate(14, handlers.page_fault, 0x0f);
        self.set_gate(16, handlers.coprocessor_error, 0x0f);
        self.set_gate(17, handlers.alignment_check, 0x0f);
        self.set_gate(18, handlers.machine_check, 0x0f);
        self.set_gate(19, handlers.simd_exception, 0x0f);

        self.set_gate(0x20, handlers.time, 0x0e);
        self.set_gate(0x21, handlers.keyboard, 0x0e);
        self.set_gate(0x2e, handlers.disk, 0x0e);
        self.set_gate(0x80, handlers.system_call, 0x0f);
        self.set_gate(0x27, handlers.master_irq7, 0x0e);
    }
}

pub struct MachineIdtHandlers {
    divide_error: u32,
    debug: u32,
    nmi: u32,
    breakpoint: u32,
    overflow: u32,
    bound: u32,
    invalid_opcode: u32,
    device_not_available: u32,
    double_fault: u32,
    coprocessor_segment_overrun: u32,
    invalid_tss: u32,
    segment_not_present: u32,
    stack_segment_error: u32,
    general_protection: u32,
    page_fault: u32,
    coprocessor_error: u32,
    alignment_check: u32,
    machine_check: u32,
    simd_exception: u32,
    time: u32,
    keyboard: u32,
    disk: u32,
    system_call: u32,
    master_irq7: u32,
}

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
pub extern "C" fn _init_idt(handlers: *const MachineIdtHandlers) {
    let handlers = unsafe { handlers.as_ref().expect("_init_idt null handlers") };
    IDT.with_mut(|idt| idt.init_gates(handlers));
}

#[no_mangle]
pub extern "C" fn _init_gdt() {
    GDT.with_mut(|gdt| gdt.init());
    TSS.with_mut(|tss| tss.init());
    GDT.with_mut(|gdt| gdt.init_tss_descriptor());
}

#[no_mangle]
pub extern "C" fn _load_idt() {
    IDT.with(|idt| {
        let idtr = DescriptorTableRegister::new(idt as *const Idt, size_of::<Idt>());
        unsafe {
            asm!(
                "lidt [{idtr}]",
                idtr = in(reg) &idtr,
                options(readonly, nostack, preserves_flags),
            );
        }
    });
}

#[no_mangle]
pub extern "C" fn _load_gdt() {
    GDT.with(|gdt| {
        let gdtr = DescriptorTableRegister::new(gdt as *const Gdt, size_of::<Gdt>());
        unsafe {
            asm!(
                "lgdt [{gdtr}]",
                gdtr = in(reg) &gdtr,
                options(readonly, nostack, preserves_flags),
            );
        }
    });
}

#[no_mangle]
pub extern "C" fn _load_task_register() {
    unsafe {
        asm!(
            "ltr {selector:x}",
            selector = in(reg) TASK_STATE_SEGMENT_SELECTOR,
            options(nostack, preserves_flags),
        );
    }
}

#[no_mangle]
pub extern "C" fn _enable_page_protection(page_directory: *const u8) {
    let page_directory_physical_address = page_directory as u32 - KERNEL_SPACE_START_ADDRESS;
    unsafe {
        asm!(
            "mov cr3, eax",
            "mov eax, cr0",
            "or eax, 0x80000000",
            "mov cr0, eax",
            inout("eax") page_directory_physical_address => _,
            options(nostack),
        );
    }
}

const fn idt_default_interrupt_handler() {
    panic!("Default Interrupt Handler!");
}

const fn idt_default_exception_handler() {
    panic!("Default Exception Handler!");
}

fn task_state_segment_ptr() -> *mut TaskStateSegment {
    TSS.get_mut() as *mut TaskStateSegment
}
