use core::{mem::size_of, ptr::NonNull};

use crate::{
    interrupt::context::TrapContext,
    mm::{phys_to_virt, KernelPages, PAGE_SIZE},
};

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

    /// 为 trap frame 预留栈顶空间，普通内核执行栈从更低地址开始使用。
    pub fn task_top(&self) -> NonNull<usize> {
        let top = self.top().addr().get() - size_of::<TrapContext>();
        NonNull::new((top & !0xf) as *mut usize).unwrap()
    }

    /// 当前进程专属 trap frame 固定放在内核栈顶端。
    pub fn trap_context(&self) -> NonNull<TrapContext> {
        NonNull::new(
            (self.top().addr().get() - size_of::<TrapContext>()) as *mut TrapContext,
        )
        .unwrap()
    }
}
