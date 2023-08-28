use riscv::register::sstatus::{self, Sstatus, SPP};
/// Trap之前的栈，详细可以去看trap.S
#[repr(C)]
pub struct TrapContext {
    /// general regs[0..31]
    /// usize在riscv64中 是64位
    pub x: [usize; 32],
    /// CSR sstatus      
    pub sstatus: Sstatus,
    /// CSR sepc
    pub sepc: usize,
    /// todo 内核页表的起始物理地址
    pub kernel_satp: usize,
    /// todo 表示当前应用在内核地址空间中的内核栈栈顶的虚拟地址
    pub kernel_sp: usize,
    /// todo 内核中trap handler入口点的虚拟地址
    pub trap_handler: usize,
}

impl TrapContext {
    /// set stack pointer to x_2 reg (sp)
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    /// init app context
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus
        // sstatus寄存器中的SPP字段用于指示处理器之前的特权级别。
        sstatus.set_spp(SPP::User); //previous privilege mode: user mode
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // app程序入口
        };
        cx.set_sp(sp); // app栈 (按照上面的定义x[2]就是 sp，让sp指向app栈)
        cx // return initial Trap Context of app
    }
}
