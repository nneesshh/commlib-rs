/// 对等节点
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Peer {
    /// peer name
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// addr such as "ip:port"
    #[prost(string, tag = "2")]
    pub raddr: ::prost::alloc::string::String,
}
/// 上行：注册对等节点
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct P2sRegisterPeer {
    /// 对等节点
    #[prost(message, optional, tag = "1")]
    pub peer: ::core::option::Option<Peer>,
}
/// 上行：注销对等节点
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct P2sUnregisterPeer {
    /// 对等节点名称
    #[prost(string, tag = "1")]
    pub peer_name: ::prost::alloc::string::String,
}
/// 下行：对等节点列表
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct S2pPeerList {
    /// 对等节点列表
    #[prost(message, repeated, tag = "1")]
    pub peer_list: ::prost::alloc::vec::Vec<Peer>,
}
/// 下行：添加对等节点通知
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct S2pPeerNotificationAdded {
    /// 对等节点
    #[prost(message, optional, tag = "1")]
    pub peer: ::core::option::Option<Peer>,
}
/// 下行：移除对等节点通知
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct S2pPeerNotificationRemoved {
    /// 对等节点名称
    #[prost(string, tag = "1")]
    pub peer_name: ::prost::alloc::string::String,
}
/// 平行：Greetings
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct P2pGreetings {
    /// 对等节点名称
    #[prost(string, tag = "1")]
    pub peer_name: ::prost::alloc::string::String,
    /// 问候语
    #[prost(string, tag = "2")]
    pub greeting: ::prost::alloc::string::String,
}
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
/// 加密 token
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct S2cEncryptToken {
    /// 64字节加密 token
    #[prost(bytes = "vec", optional, tag = "1")]
    pub token: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PairStringString {
    #[prost(string, tag = "1")]
    pub k: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub v: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InnerNodeInfo {
    #[prost(uint64, tag = "1")]
    pub nid: u64,
    #[prost(int32, tag = "2")]
    pub r#type: i32,
    #[prost(int32, repeated, tag = "3")]
    pub sids: ::prost::alloc::vec::Vec<i32>,
    #[prost(message, repeated, tag = "4")]
    pub kv: ::prost::alloc::vec::Vec<PairStringString>,
    #[prost(int32, tag = "5")]
    pub maxnum: i32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransMessageNtf {
    /// player id
    #[prost(fixed64, tag = "1")]
    pub id: u64,
    #[prost(int32, tag = "2")]
    pub cmd: i32,
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastSidMessageNtf {
    #[prost(int32, tag = "1")]
    pub sid: i32,
    #[prost(int32, tag = "2")]
    pub cmd: i32,
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    /// 指定接收渠道
    #[prost(int32, repeated, tag = "4")]
    pub channels: ::prost::alloc::vec::Vec<i32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MultiTransMessageNtf {
    /// player id
    #[prost(fixed64, repeated, tag = "1")]
    pub ids: ::prost::alloc::vec::Vec<u64>,
    #[prost(int32, tag = "2")]
    pub cmd: i32,
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// rpc 调用包
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InnerRpcCall {
    /// rpc 唯一ID
    #[prost(uint64, tag = "1")]
    pub rpc_id: u64,
    /// rpc 调用类型
    #[prost(uint64, tag = "2")]
    pub rpc_type: u64,
    /// 返回消息
    #[prost(bytes = "vec", tag = "3")]
    pub msg: ::prost::alloc::vec::Vec<u8>,
}
/// rpc 返回包
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InnerRpcReturn {
    /// rpc 唯一ID, 调用的时候确定
    #[prost(uint64, tag = "1")]
    pub rpc_id: u64,
    /// rpc 调用类型
    #[prost(uint64, tag = "2")]
    pub rpc_type: u64,
    /// 返回消息
    #[prost(bytes = "vec", tag = "3")]
    pub msg: ::prost::alloc::vec::Vec<u8>,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum InnerReservedCmd {
    IrcNone = 0,
    /// InnerNodeInfo
    IrcNodeHandshake = 1,
    /// InnerNodeInfo
    IrcNodeInfoNtf = 2,
    /// TransMessageNtf
    IrcTransMessageNtf = 3,
    /// BroadcastSidMessageNtf
    IrcBroadcastSidMessageNtf = 4,
    /// MultiTransMessageNtf
    IrcMultiTransMessageNtf = 5,
    /// rpc调用
    IrcRpcCall = 6,
    /// rpc回包
    IrcRpcReturn = 7,
    /// 跨区方法调用
    IrcCrossCall = 8,
    IrcMax = 100,
}
impl InnerReservedCmd {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            InnerReservedCmd::IrcNone => "IRC_None",
            InnerReservedCmd::IrcNodeHandshake => "IRC_NodeHandshake",
            InnerReservedCmd::IrcNodeInfoNtf => "IRC_NodeInfoNtf",
            InnerReservedCmd::IrcTransMessageNtf => "IRC_TransMessageNtf",
            InnerReservedCmd::IrcBroadcastSidMessageNtf => "IRC_BroadcastSidMessageNtf",
            InnerReservedCmd::IrcMultiTransMessageNtf => "IRC_MultiTransMessageNtf",
            InnerReservedCmd::IrcRpcCall => "IRC_RpcCall",
            InnerReservedCmd::IrcRpcReturn => "IRC_RpcReturn",
            InnerReservedCmd::IrcCrossCall => "IRC_CrossCall",
            InnerReservedCmd::IrcMax => "IRC_Max",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "IRC_None" => Some(Self::IrcNone),
            "IRC_NodeHandshake" => Some(Self::IrcNodeHandshake),
            "IRC_NodeInfoNtf" => Some(Self::IrcNodeInfoNtf),
            "IRC_TransMessageNtf" => Some(Self::IrcTransMessageNtf),
            "IRC_BroadcastSidMessageNtf" => Some(Self::IrcBroadcastSidMessageNtf),
            "IRC_MultiTransMessageNtf" => Some(Self::IrcMultiTransMessageNtf),
            "IRC_RpcCall" => Some(Self::IrcRpcCall),
            "IRC_RpcReturn" => Some(Self::IrcRpcReturn),
            "IRC_CrossCall" => Some(Self::IrcCrossCall),
            "IRC_Max" => Some(Self::IrcMax),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum RpcCmd {
    RpcNone = 0,
    /// 跨服方法调用
    RpcCrossCall = 24,
    /// 跨服方法调用回包
    RpcCrossReturn = 25,
}
impl RpcCmd {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            RpcCmd::RpcNone => "RPC_None",
            RpcCmd::RpcCrossCall => "RPC_CROSS_CALL",
            RpcCmd::RpcCrossReturn => "RPC_CROSS_RETURN",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "RPC_None" => Some(Self::RpcNone),
            "RPC_CROSS_CALL" => Some(Self::RpcCrossCall),
            "RPC_CROSS_RETURN" => Some(Self::RpcCrossReturn),
            _ => None,
        }
    }
}
