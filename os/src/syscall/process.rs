//! Process management syscalls
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::get_time_us;

/// 退出应用，并进行下一个应用
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// 暂停当前应用，并切换到下一个应用
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// milliseconds 时间
pub fn sys_get_time() -> isize {
    get_time_us() as isize
}
