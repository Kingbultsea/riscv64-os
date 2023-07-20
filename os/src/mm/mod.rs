mod heap_allocator;
mod address;
mod page_table;
mod frame_allocator;

/// 初始化堆
pub fn init() {
    heap_allocator::init_heap();
}