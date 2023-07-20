use crate::config::PAGE_SIZE;

/// 物理地址长度
const PA_WIDTH_SV39: usize = 56;
/// offset
const PAGE_SIZE_BITS: usize = 12;
/// PPN
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// 物理地址（ppn + offset）
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

/// 虚拟地址（vpn + offset）
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

/// 物理页 ppn
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

/// 虚拟页 vpn
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);


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