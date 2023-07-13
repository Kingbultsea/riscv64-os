use riscv::register::time;
use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;

const TICKS_PER_SEC: usize = 100;
const MICRO_PER_SEC: usize = 1_000_000;

// 获得mtime计数器的值(M特全级，靠SEE 即RustSBI预留接口)
pub fn get_time() -> usize {
    time::read()
}

// 设置mtimecmp，计算出 10ms 之内计数器的增量，设置下一次中断。
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

// 统计应用的运行时长，以微秒为单位返回当前计数器的值
pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / MICRO_PER_SEC)
}
