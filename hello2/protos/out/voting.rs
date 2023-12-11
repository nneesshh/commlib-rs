#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VotingRequest {
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
    #[prost(enumeration = "voting_request::Vote", tag = "2")]
    pub vote: i32,
}
/// Nested message and enum types in `VotingRequest`.
pub mod voting_request {
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Vote {
        Up = 0,
        Down = 1,
    }
    impl Vote {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Vote::Up => "UP",
                Vote::Down => "DOWN",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UP" => Some(Self::Up),
                "DOWN" => Some(Self::Down),
                _ => None,
            }
        }
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VotingResponse {
    #[prost(string, tag = "1")]
    pub confirmation: ::prost::alloc::string::String,
}
