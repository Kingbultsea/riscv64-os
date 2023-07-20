mod heap_allocator;
mod address;

/// 初始化堆
pub fn init() {
    heap_allocator::init_heap();
}