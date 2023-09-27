#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum EnumMsgType {
    None = 0,
    /// 加密 token
    EncryptToken = 1102,
}
impl EnumMsgType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            EnumMsgType::None => "None",
            EnumMsgType::EncryptToken => "EncryptToken",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "None" => Some(Self::None),
            "EncryptToken" => Some(Self::EncryptToken),
            _ => None,
        }
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct S2cEncryptToken {
    /// 64字节加密 token
    #[prost(bytes = "vec", optional, tag = "1")]
    pub token: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
}
/// 跨服消息包
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InnerCrossCall {
    /// (目标)节点类型
    #[prost(uint64, tag = "1")]
    pub node: u64,
    /// (目标)区
    #[prost(int32, tag = "2")]
    pub zone: i32,
    /// 唯一消息 ID
    #[prost(uint64, tag = "3")]
    pub id: u64,
    /// 消息类型
    #[prost(int32, tag = "4")]
    pub r#type: i32,
    /// 消息体
    #[prost(bytes = "vec", tag = "5")]
    pub msg: ::prost::alloc::vec::Vec<u8>,
    /// (源)节点类型
    #[prost(uint64, tag = "6")]
    pub source_node: u64,
    /// (源)区
    #[prost(int32, tag = "7")]
    pub source_zone: i32,
    /// 回包标记(某个消息的回包还是发起包)
    #[prost(bool, tag = "8")]
    pub resp: bool,
    /// 转发的目标所在区
    #[prost(int32, tag = "9")]
    pub trans_zone: i32,
    /// 转发的目标所在节点
    #[prost(uint64, tag = "10")]
    pub trans_node: u64,
}
