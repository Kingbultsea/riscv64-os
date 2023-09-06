//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].

mod context;
use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::{syscall::syscall, timer::set_next_trigger, task::suspend_current_and_run_next};
use crate::task::current_user_token;

use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap, Interrupt},
    stval, stvec, sie
};

// 在批处理操作系统初始化的时候，我们需要修改 stvec 寄存器来指向正确的 Trap 处理入口点。
global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
/// 在 RISC-V 架构中，stvec 寄存器用于设置中断和异常处理的向量表地址。
/// 通过将 __alltraps 函数的地址作为参数传递给 stvec::write 函数，将中断和异常处理的入口点设置为 __alltraps 函数。
/// TrapMode::Direct 表示使用直接模式，即将异常直接传递给入口点，而不进行额外的中间处理
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
/// a0寄存器保存了trap之前寄存器的内容
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            panic!("[kernel] Cannot continue!");
            //run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            panic!("[kernel] Cannot continue!");
            //run_next_app();
        }
        // 抢占式调度
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

/// sstatus.sie = 1，置0则屏蔽中断
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

pub use context::TrapContext;

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

#[no_mangle]
/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = trap_cx_ptr, reg a1 = phy addr of usr page table,
/// finally, jump to new addr of __restore asm function
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }

    // 地址相减得出 __restore内存大小
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",             // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        );
    }
}
