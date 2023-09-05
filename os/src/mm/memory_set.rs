use crate::config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE};
use crate::sync::UPSafeCell;

use super::address::{PhysAddr, PhysPageNum, StepByOne, VPNRange, VirtAddr, VirtPageNum};
use super::frame_allocator::{frame_alloc, FrameTracker};
use super::page_table::{PTEFlags, PageTable, PageTableEntry};

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use bitflags::bitflags;
use lazy_static::*;
use riscv::register::satp;

extern "C" {
    /// 内核中的内存布局 .stext段地址
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

lazy_static! {
    /// 内核地址空间实例
    /// Arc提供共享引用
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = Arc::new(unsafe {
        UPSafeCell::new(MemorySet::new_kernel())
    });
}

/// 逻辑段：一段连续地址的虚拟内存
pub struct MapArea {
    /// 虚拟页号的连续区间
    vpn_range: VPNRange,
    /// BTreeMap键值对容器，vpn -> ppn 映射
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        // 上下取整
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();

        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    /// 复制data内容到当前连续地址段下，每次4kb
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }

    /// 建立vpn与ppn映射
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            // 恒等映射
            // todo 暂定认为：假如vpn当作ppn使用，frame_alloc后续也使用了同一个ppn会出bug
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }

            // 随机映射，额外申请多一个实际的ppn地址
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }

        // 创建pte标识为
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();

        // 通过vpn寻找或创建pte，即pte地址上保存了ppn
        page_table.map(vpn, ppn, pte_flags);
    }

    #[allow(unused)]
    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {
            self.unmap_one(page_table, vpn)
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }
    
    #[allow(unused)]
    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {
            self.map_one(page_table, vpn)
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }
}

/// 虚拟内存 映射 物理内存的方式
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    /// 恒等映射
    Identical,
    /// 虚地址与物理地址的映射关系是相对随机的
    Framed,
}

bitflags! {
    /// 控制访问方式
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

/// 地址空间: 一系列有关联的不一定连续的逻辑段
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// 初始化，创建一个新的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    // todo
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    // todo
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// 把data内容推进map_area，连续空间段中，page_table的作用为寻找连续空间段的位置
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }

    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    #[allow(unused)]
    /// feamed方式插入
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // 跳板初始化
        memory_set.map_trampoline();
        // 内核section
        println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        println!("mapping .text section");
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        println!("mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        println!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set
    }

    #[allow(unused)]
    /// 1. 申请一个ppn，作为应用程序的根pte，4kb
    /// 2. TRAMPOLINE常量，即usize::Max - 4096，映射至strampoline（在链接文件中定义），todo 需要查看这样设置跳板是否正常
    /// 3. 使用xmas_elf工具分析elf文件，获取虚拟地址和内容大小，创建maparea（即虚拟地址范围）
    /// 4. 通过根pte，建立虚拟地址范围下的vpn与ppn映射，为每个vpn申请一个frame_track，4kb
    /// 5. 以粒度为4kb大小，放进申请的ppn中
    /// 6. 申请8kb用户栈，和步骤4是一样的
    /// 7. 从顶部strampoline下方，申请4kb，后续用于存放trap_contxt
    /// 8. 返回用户栈va地址，应用入口地址va，用户地址空间memory_set（内存管理器）
    /// 内存分布如下（三级pte的ppn为实际内存页）：
    /// 一级pte
    /// 二级pte
    /// (... 一共map_area个三级pte)
    /// 4kb 用户栈
    /// 4kb 用户栈
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        // 申请了一个root_ppn, 4kb，即一个frame_tracker
        let mut memory_set = Self::new_bare();

        // 申请跳板内存
        memory_set.map_trampoline();

        // 利用xmas_elf工具处理elf数据
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();

        // 判断elf是否合法
        // Magic: (7F 45 4C 46)
        // 魔数 (Magic) 独特的常数，存放在 ELF header 的一个固定位置。当加载器将 ELF 文件加载到内存之前，
        // 通常会查看 该位置的值是否正确，来快速确认被加载的文件是不是一个 ELF
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");

        // 得到 program header 的数目，然后遍历所有的 program header 并将合适的区域加入到应用地址空间中
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();

            // 类型为Load，则表示有必要被加载到内核中
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                // 计算区域在应用地址空间中的位置
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();

                // 区域访问方式
                // Program Header 提供了 ELF 文件在内存中加载和执行所需的关键信息，它是操作系统加载可执行文件的重要依据。
                // 通过解析 Program Header，操作系统可以正确地加载可执行文件，分配内存，建立程序的运行环境，并执行其中的代码。
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }

                // 这里如果并发的话，一定要管内存相对位置的，不然虚拟地址都是同一个，用了同一块物理内存，就会导致程序错误

                // 为应用申请一段连续内存段（并没有实际分配）
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

                // 向下取整后的end_va
                max_end_vpn = map_area.vpn_range.get_end();

                // 分配实际内存
                memory_set.push(
                    map_area,
                    // header
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();

        // 保护页面 4kb
        user_stack_bottom += PAGE_SIZE;

        // 建立用户栈
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // 存放trap上下文
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        (
            // 地址空间
            memory_set,
            // 用户栈虚拟地址
            user_stack_top,
            // 应用入口地址
            elf.header.pt2.entry_point() as usize,
        )
    }

    // 分页模式激活
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            // 目前用的是恒等映射，所以切换satp的指令和下一条指令是相邻的
            satp::write(satp);
            // 删除旧的快表
            asm!("sfence.vma");
        }
    }

    #[allow(unused)]
    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.shrink_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }

    #[allow(unused)]
    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.append_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }
}

/// 检测内核地址空间的多级页表是否被正确设置
#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();

    // 检测.text，不允许被写入
    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);
    // 检测.rodata，不允许被写入
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);
    // 检测.data，不允许从数据段上取指执行
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);
    println!("remap_test passed!");
}
