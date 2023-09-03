/// 初始化rust堆位置
use buddy_system_allocator::LockedHeap;
use crate::config::KERNEL_HEAP_SIZE;

// 通过 #[global_allocator] 属性，你可以告诉 Rust 在全局范围内使用你指定的自定义内存分配器，而不是默认的分配器
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

// u8 8bit
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// 初始化内核堆位置
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}
