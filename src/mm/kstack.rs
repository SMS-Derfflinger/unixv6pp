use core::ptr::NonNull;

use crate::mm::{phys_to_virt, KernelPages, PAGE_SIZE};

pub struct KernelStack {
    pages: KernelPages,
}

/// 内核栈大小：order=3 即 2^3 = 8 页 = 32KB
const KSTACK_ORDER: u32 = 3;
const KSTACK_SIZE: usize = (1 << KSTACK_ORDER) * PAGE_SIZE;

impl KernelStack {
    pub fn new() -> Self {
        Self {
            pages: KernelPages::alloc(KSTACK_ORDER),
        }
    }

    /// 返回内核栈顶的虚拟地址（栈从高地址向低地址增长）
    pub fn top(&self) -> NonNull<usize> {
        NonNull::new(
            phys_to_virt(self.pages.phys())
                .wrapping_add(KSTACK_SIZE)
                .cast(),
        )
        .unwrap()
    }
}
