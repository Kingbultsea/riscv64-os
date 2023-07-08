# 代码段标记为名为 ".text.entry" 的段，通常用于存放程序的入口代码。
    .section .text.entry
# 将符号 "_start" 声明为全局可见，表示该符号是程序的入口点。
    .globl _start
# 定义了一个标签 "_start"，表示程序的起始位置
_start:
# 将栈指针（sp）设置为 "boot_stack_top" 的地址。使用 "la"（load address）指令加载 "boot_stack_top" 的地址，并将其存储到 sp 寄存器中。
    la sp, boot_stack_top
# 调用 "rust_main" 函数。使用 "call" 指令跳转到 "rust_main" 函数，并将控制权传递给该函数。
    call rust_main

# 将下面的代码段标记为名为 ".bss.stack" 的段，通常用于存放未初始化的全局变量或堆栈空间。
    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
# 为堆栈分配一块空间，大小为 4096 * 16 字节。使用 ".space" 指令分配一定大小的空间，这里是 4096 字节（4 KB）乘以 16，共计 64 KB
    .space 4096 * 16
# 将符号 "boot_stack_top" 声明为全局可见，表示该符号是堆栈的顶部(la sp, boot_stack_top)
    .globl boot_stack_top
boot_stack_top: