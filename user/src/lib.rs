#![no_std]
#![feature(linkage)]
// #![feature(linkage)] 是 Rust 中的一个编译器属性（attribute），用于启用 linkage 功能。它允许在 Rust 代码中自定义符号的链接属性。
// 在 Rust 中，默认情况下，函数和静态变量的链接属性是 external，这意味着它们可以被其他代码访问和链接。而使用 linkage 功能可以更改链接属性，允许在编写某些特殊类型的代码时进行自定义。
// 具体来说，#![feature(linkage)] 允许使用 #[linkage = "..."] 这样的语法来指定函数或静态变量的链接属性。这样可以更灵活地控制代码的链接行为，例如将函数声明为 extern "C"，或者指定特定平台的链接属性等。
// 需要注意的是，#![feature(linkage)] 是一个 unstable（不稳定）的功能，只能在使用 nightly 版本的 Rust 编译器时才能启用。
#![feature(panic_info_message)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
// 使用 Rust 的宏将其函数符号 main 标志为弱链接。这样在最后链接的时候，虽然在 lib.rs 和 bin 目录下的某个应用程序都有 main 符号，
// 但由于 lib.rs 中的 main 符号是弱链接，链接器会使用 bin 目录下的应用主逻辑作为 main 。
// 这里主要是进行某种程度上的保护，如果在 bin 目录下找不到任何 main ，那么编译也能够通过，但会在运行时报错
fn main() -> i32 {
    panic!("Cannot find main!");
}

fn clear_bss() {
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    (start_bss as usize..end_bss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}

use syscall::*;

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}
pub fn yield_() -> isize {
    sys_yield()
}
pub fn get_time() -> isize {
    sys_get_time()
}