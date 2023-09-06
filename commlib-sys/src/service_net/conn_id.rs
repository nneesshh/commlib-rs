use bytemuck::NoUninit;

/// Connection id
#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash, NoUninit)]
#[repr(C)]
pub struct ConnId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl ConnId {}

impl From<usize> for ConnId {
    #[inline(always)]
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}

// 为了使用 `{}` 标记，必须手动为类型实现 `fmt::Display` trait。
impl std::fmt::Display for ConnId {
    // 这个 trait 要求 `fmt` 使用与下面的函数完全一致的函数签名
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // 仅将 self 的第一个元素写入到给定的输出流 `f`。返回 `fmt:Result`，此
        // 结果表明操作成功或失败。注意 `write!` 的用法和 `println!` 很相似。
        write!(f, "{}", self.id)
    }
}
