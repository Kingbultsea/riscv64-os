use alloc::vec::Vec;
use lazy_static::*;
use crate::config::MEMORY_END;

/// 物理页帧管理器
use super::address::{PhysPageNum, PhysAddr};

use crate::sync::UPSafeCell;
lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<StackFrameAllocator> = unsafe {
        UPSafeCell::new(StackFrameAllocator::new())
    };
}

trait FrameAllocator {
    fn new() -> Self;
    /// 分配一个物理页
    fn alloc(&mut self) -> Option<PhysPageNum>;
    /// 释放目标物理页
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 【current, end)，代表此前从未被分配出去过的物理页
pub struct StackFrameAllocator {
    // 空闲内存的起始物理页编号
    current: usize,
    // 空闲内存的结束物理页号
    end: usize,
    // 保存了被回收的物理页编号
    recycled: Vec<usize>, 
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        // 从已被回收过的内存中再进行分配
        if let Some(ppn) = self.recycled.pop() {
            // todo考虑是否清0
            Some(ppn.into())
        } else {
            // 无法分配，内存范围外
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;

        // 检测是否已经被回收过
        if ppn >= self.current || self.recycled.iter().find(|&v| { *v == ppn }).is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }

        self.recycled.push(ppn);
    }
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.current = r.0;
    }
}

pub fn init_frame_allocator() {
    extern "C" {
        // 内核内存边界
        fn ekernel();
    }
    FRAME_ALLOCATOR
        .exclusive_access()
        .init(PhysAddr::from(ekernel as usize).ceil(), PhysAddr::from(MEMORY_END).floor());
}

pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // 根据ppn地址获取指针（4kb内容）
        let bytes_array = ppn.get_bytes_array();
        // 清空ppn的内容
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

/// drop后需要回收
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

/// 公开给外部使用的内存管理器
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR
        .exclusive_access()
        .dealloc(ppn);
}
