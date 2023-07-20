/// 物理页帧管理器
use super::address::PhysPageNum;

trait FrameAllocator {
    fn new() -> Self;
    /// 分配一个物理页
    fn alloc(&mut self) -> Option<PhysPageNum>;
    /// 释放目标物理页
    fn dealloc(&mut self, ppn: PhysPageNum);
}