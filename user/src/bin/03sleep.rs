#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::yield_;
use user_lib::get_time;

#[no_mangle]
fn main() -> i32 {
    let current_timer = get_time();
    let wait_for = current_timer + 3000;
    while get_time() < wait_for {
        yield_();
    }
    println!("Test sleep OK!");
    0
}
