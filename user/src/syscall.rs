use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;

// s0 -> s11函数是保存寄存器
// s0是sp寄存器，用于debugger
// https://jborza.com/post/2021-05-11-riscv-linux-syscalls/
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        // x10~x17 : 对应 a0~a7
        asm!(
            // Environment Call 用户态程序和操作系统之间进行通讯
            "ecall",
            inlateout("x10") args[0] => ret,
            // 把args[1]传递进a1寄存器中
            in("x11") args[1],
            in("x12") args[2],
            // 系统调用id
            in("x17") id
        );
    }
    ret
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    // a0为字符串地址 这里的fd为1
    // a1 为字符串长度 
    // a7 为 64 
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}
