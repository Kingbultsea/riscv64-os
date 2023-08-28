//! Uniprocessor interior mutability primitives

use core::cell::{RefCell, RefMut};

// 只要复合类型中有一个成员不是Send或Sync，那么该复合类型也就不是Send或Sync
// 裸指针两者都没实现，因为它本身就没有任何安全保证
// UnsafeCell不是Sync，因此Cell和RefCell也不是
// Rc两者都没实现(因为内部的引用计数器不是线程安全的)
// 这里的封装仅仅是尽量减少代码中unsafe的使用，调用exclusive_access借用直接修改即可
// 需要自己保证
pub struct UPSafeCell<T> {
    /// inner data
    inner: RefCell<T>,
}

// 实现Send的类型可以在线程间安全的传递其所有权
// 实现Sync的类型可以在线程间安全的共享(通过引用)
unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    // 通过使用 unsafe 关键字，Rust 鼓励程序员明确知晓代码中的不安全性，并且在编译期间进行必要的审查。
    // 在使用 unsafe 块时，开发者需要仔细考虑代码的正确性，并且采取适当的措施来确保代码的安全性。
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
