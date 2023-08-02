//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2; // 8kb
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 8kb
pub const MAX_APP_NUM: usize = 4;
pub const APP_BASE_ADDRESS: usize = 0x8040_0000;
// 8bit * 0x20000
// 128KB
pub const APP_SIZE_LIMIT: usize = 0x20000;

/// 跳板：4kb = 内存顶部 - 4kb
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// TRAP_CONTEXT：4kb（陷阱上下文）是用来保存进程或线程在发生异常或系统调用（陷阱）时的上下文信息的数据结构。当一个进程或线程发生异常或执行系统调用时，
/// 操作系统会将当前的 CPU 寄存器状态和其他相关的上下文信息保存到 TRAP_CONTEXT 中，然后进入内核态（内核模式）处理异常或系统调用。
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

/// 内核堆大小 3145728 = 3mb
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;

/// qemu总内存限制在8mb
pub const MEMORY_END: usize = 0x80800000;

/// 1_0000_0000_0000 13位
pub const PAGE_SIZE: usize = 4096;

pub use crate::board::CLOCK_FREQ;