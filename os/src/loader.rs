//! Loading user applications into memory
//!
//! For chapter 3, user applications are simply part of the data included in the
//! kernel binary, so we only need to copy them to the space allocated for each
//! app to load them. We also allocate fixed spaces for each task's
//! [`KernelStack`] and [`UserStack`].

use crate::config::*;
use crate::trap::TrapContext;
use core::arch::asm;

#[repr(align(4096))] // 以4096对齐
#[derive(Copy, Clone)]
struct KernelStack {
    // 8bit 代表一个地址
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))] // 以 4096对齐
#[derive(Copy, Clone)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, trap_cx: TrapContext) -> usize {
        // 用户栈压进内核栈中
        let trap_cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_cx_ptr = trap_cx;
        }
        trap_cx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/// Get base address of app i.
fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

/// Get the total number of applications.
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }

    //当前内存块为3
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// Load nth user app at
/// [APP_BASE_ADDRESS + n * APP_SIZE_LIMIT, APP_BASE_ADDRESS + (n+1) * APP_SIZE_LIMIT).
pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();

    // core::slice::from_raw_parts函数的元素大小由其参数类型决定。
    // 例如，如果传递给core::slice::from_raw_parts的指针类型为*const u8，那么元素的大小就是1字节
    // num_app_ptr.add(1)指向下一个（8 * 8 = 64） - 4块
    // app_0_start 64
    // app_1_start 64
    // app_2_start 64
    // app_2_end   64
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };

    // OS 将修改会被 CPU 取指的内存区域，这会使得 i-cache 中含有与内存中不一致的内容
    unsafe {
        // 汇编指令
        asm!("fence.i");
    }
    // load apps
    for i in 0..num_app {
        let base_i = get_base_i(i);
        // clear region
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
        // 从地址上加载，（app_start[i + 1] - app_start[i]）两个地址之间 可以算出内存大小
        let len = app_start[i + 1] - app_start[i];
        let src = unsafe { core::slice::from_raw_parts(app_start[i] as *const u8, len) };
        println!(
            "\r\nloading app {}\r\nsize: {}kb \r\napp_ptr: {:x}\r\napp_end: {:x}",
            i,
            len,
            app_start[i],
            app_start[i + 1],
        );
        // 把二进制加载到指定入口 (0x80400000) - (0x80420000)
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        dst.copy_from_slice(src);
    }
}

/// get app info with entry and sp and save `TrapContext` in kernel stack
pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        // 应用代码入口
        get_base_i(app_id),
        // 用户栈
        USER_STACK[app_id].get_sp(),
    ))
}

// 0x8020aef0
// addi sp, sp, 272 增加栈后
// 0x8020b000