/// 页表项
use bitflags::*;
use super::address::PhysPageNum;

bitflags! {
    /// 44位PPN + 2位RSW + 8位Flags
    pub struct PTEFlags: u8 {
        // 仅当位1时，页表项才是合法的
        const V = 1 << 0;
        // Read
        const R = 1 << 1;
        // Write
        const W = 1 << 2;
        // 是否为可执行页面
        const X = 1 << 3;
        // 控制索引到这个页表项的对应虚拟页面是否在 CPU 处于 U 特权级的情况下是否被允许访问
        const U = 1 << 4;
        // 共享页表项，多线程会用到
        const G = 1 << 5;
        // 处理器记录自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被访问过
        const A = 1 << 6;
        // 处理器记录自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被修改过
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry { bits: ppn.0 << 10 | flags.bits as usize }
    }

    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1) ).into()
    }

    pub fn flags(&self) -> PTEFlags {
        // 取8位，用除法也可以（self.bits / ）
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// 检测V位是否合法（1合法）
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
}