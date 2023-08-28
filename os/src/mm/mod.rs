mod heap_allocator;
mod address;
mod page_table;
mod frame_allocator;
mod memory_set;

pub use memory_set::KERNEL_SPACE;

pub fn init() {
    // 内核初始化堆
    heap_allocator::init_heap();
    // 内存分配管理器初始化（内存范围）
    frame_allocator::init_frame_allocator();
    // 开启分页模式
    KERNEL_SPACE.exclusive_access().activate();
}