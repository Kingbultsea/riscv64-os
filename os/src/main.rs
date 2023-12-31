//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`task`]: Task management
//! - [`syscall`]: System call handling and implementation
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_first_task()`] and for the first time go to
//! userspace.

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
// 动态内存处理失败，需要panic
#![feature(alloc_error_handler)]
extern crate alloc;

use core::arch::global_asm;

// 可以不像平时那样建立mod.rs
#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
mod mm;
mod sbi;
mod sync;
pub mod syscall;
pub mod task;
mod timer;
pub mod trap;

// .asm 文件则通常是纯粹的原始汇编文件，不包含预处理器指令。这种文件直接包含原始的汇编指令，没有经过额外的处理或转换。
global_asm!(include_str!("entry.asm"));
// .S 文件中可以包含预处理指令（如 #include 和 #define），这意味着该文件将通过预处理器进行处理，并允许使用宏定义、
// 条件编译等高级特性。预处理器将处理这些指令，并生成最终的汇编代码。
global_asm!(include_str!("link_app.S"));

/// 内核需要bss初始化为0,bss用于储存未初始化的全局或静态变量
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

/// the rust entry-point of os
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world! {}", timer::get_time_us());

    // 初始化内存分配相关的工作
    mm::init();

    // println!("[kernel] back to world!");
    // mm::remap_test();

    // 指定trap触发函数，开启S模式下的trap
    trap::init();

    // loader::load_apps();

    // 防止S特权级时钟中断被屏蔽，需要进行初始化
    trap::enable_timer_interrupt();

    // 触发Trap::Interrupt(Interrupt::SupervisorTimer)，内部继续调用set_next_trigger，以达到10ms中断一次的效果
    timer::set_next_trigger();

    task::run_first_task();
    panic!("Unreachable in rust_main!");
}
