//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::sync::UPSafeCell;
use lazy_static::*;
use alloc::vec::Vec;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// 总应用数
    num_app: usize,
    /// 管理所有任务块，记录当前运行的应用id
    /// 任何对于 static mut 变量的访问控制都是 unsafe 的，而我们要在编程中尽量避免使用 unsafe ，这样才能让编译器负责更多的安全性检查。
    inner: UPSafeCell<TaskManagerInner>,
}

/// 需要设置一个inner是因为
pub struct TaskManagerInner {
    /// 任务块
    tasks: Vec<TaskControlBlock>,
    // tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

// lazy_static! {
//     /// Global variable: TASK_MANAGER
//     pub static ref TASK_MANAGER: TaskManager = {
//         let num_app = get_num_app();
//         // 保存每个任务的寄存器
//         let mut tasks = [TaskControlBlock {
//             task_cx: TaskContext::zero_init(),
//             task_status: TaskStatus::UnInit,
//         }; MAX_APP_NUM];

//         for (i, task) in tasks.iter_mut().enumerate() {
//             // 返回每个app的trap_context指针
//             task.task_cx = TaskContext::goto_restore(init_app_cx(i));
//             task.task_status = TaskStatus::Ready;
//         }
//         TaskManager {
//             num_app,
//             inner: unsafe {
//                 UPSafeCell::new(TaskManagerInner {
//                     tasks,
//                     current_task: 0,
//                 })
//             },
//         }
//     };
// }

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;

        // ra: trap_return
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;

        // 如果不手动 drop 的话，编译器会在 __switch 返回时，也就是当前应用被切换回来的时候才 drop，
        // 这期间都不能修改 TaskManagerInner ，甚至不能读（因为之前是可变借用），会导致内核 panic 报错退出
        drop(inner);

        // 充当填充参数
        // todo 可以传递多一个参数去asm判断
        let mut _unused = TaskContext::zero_init();

        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    // todo
    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// 将当前任务状态标记为TaskStatus::Ready
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// 将当前任务状态标记为TaskStatus::Exited
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// 寻找为Ready的应用
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;

        // 因为不会包括最后一位数，需要+1
        (current + 1..current + self.num_app + 1)
            // 取余 循环一圈 1 -> 2 -> 0
            .map(|id| {
                println!("raw id {}", id);
                id % self.num_app
            })
            .find(|id| {
                let status = inner.tasks[*id].task_status;
                println!(
                    "running app id {} status: {:?} app_num: {} start: {} end: {}",
                    id,
                    status,
                    self.num_app,
                    current + 1,
                    current + self.num_app
                );
                status == TaskStatus::Ready
            })
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            println!("All applications completed!");
            use crate::board::QEMUExit;
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }
    }
}

/// run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// rust next task
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// suspend current task
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// exit current task
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
