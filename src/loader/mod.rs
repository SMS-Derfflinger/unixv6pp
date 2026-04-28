use alloc::{boxed::Box, vec, vec::Vec};
use eonix_mm::paging::PAGE_SIZE;

use crate::{
    compat::compat_flush_page_directory,
    fs::InodeRef,
    machine::{global_user_page_table, EntryFlags},
    sync::SpinExt,
    user::MemoryDescriptor,
    Ext,
};

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LSB: u8 = 1;
const ELF_ET_EXEC: u16 = 2;
const ELF_EM_RISCV: u16 = 243;
const ELF_PT_LOAD: u32 = 1;

const PF_X: u32 = 1 << 0;
const PF_W: u32 = 1 << 1;

const DEFAULT_STACK_SIZE: usize = 0x10_000;
const PTES_PER_PAGE: usize = PAGE_SIZE / size_of::<usize>();

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct ElfHeader64 {
    ident: [u8; 16],
    elf_type: u16,
    machine: u16,
    version: u32,
    entry: u64,
    phoff: u64,
    shoff: u64,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct ProgramHeader64 {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct LoadSegment {
    offset: usize,
    vaddr: usize,
    filesz: usize,
    memsz: usize,
    flags: u32,
}

impl LoadSegment {
    fn is_writable(&self) -> bool {
        (self.flags & PF_W) != 0
    }

    fn end(&self) -> usize {
        self.vaddr + self.memsz
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct ELFParser {
    pub entry: usize,
    pub text: usize,
    pub text_len: usize,

    pub data: usize,
    pub data_len: usize,

    pub stack_size: usize,
    pub heap_size: usize,

    segments: Option<Box<[LoadSegment]>>,
}

impl ELFParser {
    pub fn new() -> Self {
        Self {
            stack_size: DEFAULT_STACK_SIZE,
            ..Default::default()
        }
    }

    pub fn load(&mut self, inode: &InodeRef) -> bool {
        let mut header = ElfHeader64::default();
        if inode.lock().read(header.as_buffer(), 0).is_err() {
            return false;
        }

        if header.ident[..4] != ELF_MAGIC
            || header.ident[4] != ELF_CLASS_64
            || header.ident[5] != ELF_DATA_LSB
            || header.elf_type != ELF_ET_EXEC
            || header.machine != ELF_EM_RISCV
            || header.phentsize as usize != size_of::<ProgramHeader64>()
            || header.phnum == 0
        {
            return false;
        }

        let mut phdrs = vec![ProgramHeader64::default(); header.phnum as usize];
        if inode
            .lock()
            .read(phdrs.as_mut_slice().as_buffer(), header.phoff as usize)
            .is_err()
        {
            return false;
        }

        let mut segments = Vec::new();
        for phdr in phdrs {
            if phdr.p_type != ELF_PT_LOAD {
                continue;
            }

            let filesz = phdr.p_filesz as usize;
            let memsz = phdr.p_memsz as usize;
            if filesz > memsz {
                return false;
            }

            segments.push(LoadSegment {
                offset: phdr.p_offset as usize,
                vaddr: phdr.p_vaddr as usize,
                filesz,
                memsz,
                flags: phdr.p_flags,
            });
        }

        if segments.is_empty() {
            return false;
        }

        segments.sort_by_key(|segment| segment.vaddr);

        let text_start = segments[0].vaddr;
        let first_writable = segments.iter().find(|segment| segment.is_writable());
        let max_load_end = segments
            .iter()
            .map(LoadSegment::end)
            .max()
            .unwrap_or(text_start);
        let data_start = first_writable.map(|segment| segment.vaddr).unwrap_or(max_load_end);
        let data_end = segments
            .iter()
            .filter(|segment| segment.is_writable())
            .map(LoadSegment::end)
            .max()
            .unwrap_or(data_start);

        if text_start >= MemoryDescriptor::USER_SPACE_END
            || data_end > MemoryDescriptor::USER_SPACE_END
            || self.entry >= MemoryDescriptor::USER_SPACE_END
        {
            return false;
        }

        self.entry = header.entry as usize;
        self.text = text_start;
        self.text_len = data_start.saturating_sub(text_start);
        self.data = data_start;
        self.data_len = data_end.saturating_sub(data_start);
        self.heap_size = 0;
        self.stack_size = DEFAULT_STACK_SIZE;
        self.segments = Some(segments.into_boxed_slice());

        true
    }

    pub fn relocate(&mut self, inode: &InodeRef, shared_text: bool) {
        let Some(segments) = self.segments.as_deref() else {
            return;
        };

        if !shared_text {
            self.set_text_write(true);
        }

        for segment in segments {
            if shared_text && !segment.is_writable() {
                continue;
            }

            unsafe {
                (segment.vaddr as *mut u8).write_bytes(0, segment.memsz);
            }

            if segment.filesz == 0 {
                continue;
            }

            let dst = unsafe {
                core::slice::from_raw_parts_mut(segment.vaddr as *mut u8, segment.filesz)
            };

            if inode.lock().read(dst, segment.offset).is_err() {
                return;
            }
        }

        if !shared_text {
            self.set_text_write(false);
        }
    }

    fn set_text_write(&self, writable: bool) {
        if self.text_len == 0 {
            return;
        }

        let start_page = self.text / PAGE_SIZE;
        let page_count = align_up(self.text_len, PAGE_SIZE) / PAGE_SIZE;
        let user_pts = global_user_page_table();

        for page_idx in start_page..start_page + page_count {
            let table_idx = page_idx / PTES_PER_PAGE;
            let pte_idx = page_idx % PTES_PER_PAGE;
            let pte = &mut user_pts[table_idx][pte_idx];
            let (pfn, mut flags) = pte.get();
            if !flags.contains(EntryFlags::VALID) {
                continue;
            }

            if writable {
                flags |= EntryFlags::WRITE | EntryFlags::DIRTY;
            } else {
                flags.remove(EntryFlags::WRITE);
            }

            pte.set(Some(pfn), flags);
        }

        compat_flush_page_directory();
    }
}

fn align_up(value: usize, align: usize) -> usize {
    if value == 0 {
        0
    } else {
        (value + align - 1) & !(align - 1)
    }
}
