/// 页表项

use bitflags::*;
use super::{address::{PhysPageNum, VirtPageNum, VirtAddr, StepByOne}, frame_allocator::{FrameTracker, frame_alloc}};
use alloc::vec;
use alloc::vec::Vec;

bitflags! {
    // 8位flags（54 pte中的8位）
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
/// 44位PPN + 2位RSW + 8位Flags
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

    /// pte转换为ppn，sv39第三级的pte的ppn
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

    /// 检测可读性
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    /// 检测写入权限
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    /// 检测是否可执行
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// 页表节点
/// 每个应用的地址空间都对应一个不同的多级页表，不同页表的起始地址（即页表根节点的地址）是不一样的
/// root_ppn 作为页表唯一的区分标志
pub struct PageTable {
    root_ppn: PhysPageNum,

    /// 保留所有的节点，包括根节点，不包括结点
    frames: Vec<FrameTracker>,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
}

/// vpn与ppn的映射，key为vpn
/// vpn: 结点
impl PageTable {
    /// 建立映射，在所有操作后，再刷新tlb，映射不做刷新，避免耗费不必要的开销
    /// 即找到结点，并完善pte = ppn + flags + rsw
    /// 等于强行修改ppn了
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        // 初始化结点pte
        if let Some(pte) = self.find_pte_crate(vpn) {
            *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
        }
    }

    #[allow(unused)]
    /// 取消映射
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        if let Some(pte) = self.find_pte(vpn) {
            // 清空空间即可
            *pte = PageTableEntry::empty();
        }
    }

    /// 从根节点向下寻找所有节点，如无则创建一块物理页ppn，最后一级将返回结点 pte
    /// 图示：http://rcore-os.cn/rCore-Tutorial-Book-v3/_images/sv39-full.png
    fn find_pte_crate(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for i in 0..3 {
            // idexs[i] 的值 就是pte的索引
            // step1： 根据vpn，从ppn里面内存中寻找pte，
            let pte = &mut ppn.get_pte_array()[idxs[i]];

            // step2：如果pte不存在，则申请一个ppn，再等下一次循环的时候，把pte
            if !pte.is_valid() {
                // 申请一个物理页ppn
                let frame = frame_alloc().unwrap();

                // pte 中 存入一个 ppn
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);

                // 把申请的节点推进去记录
                self.frames.push(frame);
            }

            // 结点，直接返回pte
            if i == 2 {
                result = Some(pte);
                break;
            }

            // pte转换为ppn，继续寻找下一级，后续会根据该ppn寻找下一级索引位置
            ppn = pte.ppn();
        }

        result
    }

    /// 当找不到合法的pte，不会去创建，直接返回None，其余和find_pte_crate一样
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;

        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idxs[i]];

            if !pte.is_valid() {
                return None;
            }

            if i == 2 {
                result = Some(pte);
                break;
            }

            ppn = pte.ppn();
        }

        result
    }

    // 查表方式：当遇到需要查一个特定页表（非当前正处在的地址空间的页表时），
    // 便可先通过 PageTable::from_token 新建一个页表，再调用它的 translate 方法查页表。

    #[allow(unused)]
    /// stap: mode 4 + asid 16 + ppn 44
    /// 从satp中获取ppn
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// vpn转换为pte，但返回的是clone过的pte
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| { pte.clone() })
    }

    /// 0x8000_0000_0000_0000 | self.root_ppn.0
    /// mode 4 + asid 16 + ppn 44
    /// 当mode设置为8的时候，SV39分页机制将被启用，所有 S/U 特权级的访存被视为一个 39(3 * 9 + 12) 位的虚拟地址
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}
