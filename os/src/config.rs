//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 4096 * 2; // 8kb
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; // 8kb
pub const MAX_APP_NUM: usize = 4;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
// 8bit * 0x20000
// 128KB
pub const APP_SIZE_LIMIT: usize = 0x20000;

/// 内核堆大小 3145728 = 3mb
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;

pub use crate::board::CLOCK_FREQ;