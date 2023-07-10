# 为了使用.rept
.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text
    .globl __alltraps
    .globl __restore
# 4字节对齐 是 RISC-V 特权级规范的要求
    .align 2
# 将 Trap 上下文保存在内核栈上，然后跳转到使用 Rust 编写的 trap_handler 函数完成 Trap 分发及处理-
__alltraps:
# CSR不能直接操作，当想要对 CSR 进行操作时，需要先使用读取 CSR 的指令将 CSR 读到一个通用寄存器中，而后操作该通用寄存器，最后再使用写入 CSR 的指令将该通用寄存器的值写入到 CSR 中。
# 在这一行之前 sp 指向用户栈， sscratch 指向内核栈，现在 sp 指向内核栈， sscratch 指向用户栈
# 因此这里起到的是交换 sscratch 和 sp 的效果
    csrrw sp, sscratch, sp
    # now sp->kernel stack, sscratch->user stack
    # allocate a TrapContext on kernel stack
# 在内核栈上保存 Trap 上下文，于是预先分配 34 X 8字节的栈(8字节是因为64位寄存器)（1个地址1个字节）
# 栈是从高地址向低地址增长的，因此减法操作将栈指针向下移动，为栈上的新数据腾出空间
# sp是x2寄存器（stack pointer）
    addi sp, sp, -34*8
    # save general-purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
# 保存 x5~x31 寄存器 SAVE_GP在顶部定义
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
# 将CSR status 和 spec的值分别读到寄存器t0和t1中，
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # read user stack from sscratch and save it on the kernel stack
# 专门处理sp的问题，首先将sscratch的值读到寄存器t2并保存到内核上。sscratch 的值是进入 Trap 之前的 sp 的值，指向用户栈
# 意思就是把用户栈保存到t2寄存器中
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # set input argument of trap_handler(cx: &mut TrapContext)
# 让寄存器a0指向内核栈的栈指针
    mv a0, sp
    call trap_handler

# 当 trap_handler 返回之后会从调用 trap_handler 的下一条指令开始执行，也就是从栈上的 Trap 上下文恢复的 __restore
__restore:
    # now sp->kernel stack(after allocated), sscratch->user stack
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # release TrapContext on kernel stack
    # 栈指针增加272 字节 等于不指向kernel stack的内容了
    addi sp, sp, 34*8
    # now sp->kernel stack, sscratch->user stack
    csrrw sp, sscratch, sp
    # sret是一个特权指令，用于从特权模式（如机器模式或超级用户模式）返回到先前的特权模式（如用户模式）
    # 恢复用户模式的程序计数器（PC）。
    # 恢复用户模式的堆栈指针（SP）。
    # 恢复用户模式的状态寄存器（SR）
    # CPU 会将当前的特权级按照 sstatus 的 SPP 字段设置为 U 或者 S ；
    # CPU 会跳转到 sepc 寄存器指向的那条指令，然后继续执行。
    sret

# sscratch会变成80213000 即user_stack[0]的地址
# 再吧sp指向80213000