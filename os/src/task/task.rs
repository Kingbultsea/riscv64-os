//! Types related to task management
use super::TaskContext;
use crate::trap::{trap_handler, TrapContext};
use crate::config::{kernel_stack_position, TRAP_CONTEXT};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};

// 任务控制块
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    // 应用地址空间
    pub memory_set: MemorySet,
    // 应用地址空间次高页面的Trap上下文被实际存放在物理页帧的物理页号
    pub trap_cx_ppn: PhysPageNum,
    // 统计应用数据大小，即从0x0开始到用户栈结束一共包含多少字节
    pub base_size: usize,
    pub heap_bottom: usize,
    pub program_brk: usize,
}

// 任务状态
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    // UnInit,  // 未初始化
    Ready,   // 准备运行
    Running, // 正在运行
    Exited,  // 已退出
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // 加载应用到内存中
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let task_status = TaskStatus::Ready;

        // map a kernel-stack in kernel space
        // 假如有两个应用：则内存分布为 内存顶部地址- 8kb内存 -（4kb间隔）- 8kb内存
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        let task_control_block = Self {
            task_status,
            // ra被设置为 trap_return ，任务切换__switch执行完毕后，再去执行该方法
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            // todo
            base_size: user_sp,
            // 即栈顶位置
            heap_bottom: user_sp,
            program_brk: user_sp,
        };

        // 获取trap_cx，这里是引用内存，但没有实际应用，不需要申请，from_elf的时候已经申请好，即TRAP_CONTEXT - TRAMPOLINE
        let trap_cx = task_control_block.get_trap_cx();

        // 填写实际内容
        *trap_cx = TrapContext::app_init_context(
            // 应用程序入口
            entry_point,
            // 应用栈顶
            user_sp,
            // root_ppn，设置satp的时候，需要填入root_ppn作为根pte，后续交给处理器使用va寻找到pa
            KERNEL_SPACE.exclusive_access().token(),
            // 上面创建的范围，但是实际上还没有申请对应的内存
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    /// change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }
}
