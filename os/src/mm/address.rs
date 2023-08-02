use crate::config::PAGE_SIZE;
use core::fmt::Debug;

use super::page_table::PageTableEntry;

/// va长度
const VA_WIDTH_SV39: usize = 39;
/// 物理地址长度
const PA_WIDTH_SV39: usize = 56;
/// offset
const PAGE_SIZE_BITS: usize = 12;
/// PPN
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// 物理地址（ppn 44 + offset 12），即56位
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    /// 获取12低位 offset
    pub fn page_offset(&self) -> usize { self.0 & (PAGE_SIZE - 1) }
    /// 4kb对齐，因为是usize，不会有浮点数
    pub fn floor(&self) -> PhysPageNum { PhysPageNum(self.0 / PAGE_SIZE) }
    /// 向上取整，用二进制的想法想下，只要12低位有值，则会进1（4kb对齐），usize不会计算出浮点数
    pub fn ceil(&self) -> PhysPageNum { PhysPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE) }
}

/// 虚拟地址39（vpn + offset）= (9 + 9 + 9) + 12
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);
impl VirtAddr {
    // 向下取整
    pub fn floor(&self) -> VirtPageNum {
        // 0000_0000 0000_0000 0000_0000 0000_0000 0000_0000
        // 0000_0000 0000_0000 0001_0000 0000_0000 0000_0000
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    // 向上取整
    pub fn ceil(&self) -> VirtPageNum {
        if self.0 == 0 {
            // todo
            VirtPageNum(0)
        } else {
            // 0000_0000 0100_1000 0000_0000 0000_0000 0100_0101
//PAGE_SIZE-1 =0000_0000 0000_0000 0000_1111 1111_1111 1111_1111
      // 相加后 0000_0000 0100_1000 0001_0000 0000_0000 0100_0100
// 除PAGE_SIZE 0000_0000 0100_1000 0001_0000 0000_0000 0000_0000
            VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
        }
    }
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {
        // todo
        if v.0 >= (1 << (VA_WIDTH_SV39 - 1)) {
            v.0 | (!((1 << VA_WIDTH_SV39) - 1))
        } else {
            v.0
        }
    }
}

/// 取低位39位
impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        Self(v & ((1 << VA_WIDTH_SV39) - 1))
    }
}
impl From<VirtAddr> for VirtPageNum {
    fn from(v: VirtAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}

/// 物理页 ppn
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

impl PhysPageNum {
    /// 获取一页内存（4kb）指针，粒度为1byte
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();

        // 若存在多个输入生命周期，且其中一个是 &self 或 &mut self，则 &self 的生命周期被赋给所有的输出生命周期

        // 若只有一个输入生命周期(函数参数中只有一个引用类型)，那么该生命周期会被赋给所有的输出生命周期，也就是所有返回值的生命周期都等于该输入生命周期
        
        // 该引用指向的数据活得跟程序一样久
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }

    /// 512个pte 等于 一个物理页 （512 * 64）/ 8 = 4986 = 4kb
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = self.clone().into();
        unsafe {
            core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512)
        }
    }

    /// 获取一个恰好放在一个物理页帧开头的类型为 T 的数据的可变引用
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = self.clone().into();
        unsafe {
            // core::slice::from_raw_parts_mut需要填入准确的大小，而用类型就不需要
            (pa.0 as *mut T).as_mut().unwrap()
        }
    }
}

/// 虚拟页 vpn 27 = 9 + 9 + 9 = 可以存 512个pte + 512个pte + 512个pte
/// 寻址范围为上下256GB
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct VirtPageNum(pub usize);

impl VirtPageNum {
    /// 3级页面索引
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        // 2 1 0
        for i in (0..3).rev() {
            // 取前面9位
            idx[i] = vpn & 511;
            // 去除已经被去取掉的9位
            vpn >>= 9;
        }
        idx
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}


/// 取56位地址，其余清0
impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self { Self(v & ( (1 << PA_WIDTH_SV39) - 1 )) }
}
/// 取44位地址，其余清0
impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self { Self(v & ( (1 << PPN_WIDTH_SV39) - 1 )) }
}
/// 转usize直接从元组结构体提取即可
impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self { v.0 }
}
impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self { v.0 }
}

/// 物理地址转换为PPN
impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {
        // 转换为ppn地位12需要制为0
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}

pub trait StepByOne {
    fn step(&mut self);
}

/// vpn地址+1
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone)]
/// a simple range structure for type T
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}

impl<T> SimpleRange<T> where T: StepByOne + Copy + PartialEq + PartialOrd + Debug {
    pub fn new(start: T, end: T) -> Self {
        Self {
            l: start,
            r: end
        }
    }

    /// 方便语义
    pub fn get_start(&self) -> T {
        self.l
    }

    /// 方便语义
    pub fn get_end(&self) -> T {
        self.r
    }
}

impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}


/// SimpleRange的迭代器
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

/// vpn范围
pub type VPNRange = SimpleRange<VirtPageNum>;
