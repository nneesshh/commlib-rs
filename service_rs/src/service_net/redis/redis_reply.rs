use bytemuck::NoUninit;

///
#[derive(PartialEq, Copy, Clone, NoUninit, Debug)]
#[repr(u8)]
pub enum RedisReplyType {
    Error = 0,
    BulkString = 1,
    SimpleString = 2,
    Null = 3,
    Integer = 4,
    Array = 5,
}

///
#[derive(Debug)]
pub struct RedisReply {
    rpl_type: RedisReplyType,

    //
    str_val: String,
    int_val: i64,
    arr_val: Vec<RedisReply>,
}

impl RedisReply {
    ///
    pub fn null() -> Self {
        Self {
            rpl_type: RedisReplyType::Null,

            str_val: "".to_owned(),
            int_val: 0,
            arr_val: Vec::new(),
        }
    }

    ///
    pub fn from(value: &str, rpl_type: RedisReplyType) -> Self {
        assert!(rpl_type == RedisReplyType::BulkString || rpl_type == RedisReplyType::SimpleString);
        Self {
            rpl_type,

            str_val: value.to_owned(),
            int_val: 0,
            arr_val: Vec::new(),
        }
    }

    ///
    pub fn from_error(value: &str) -> Self {
        Self {
            rpl_type: RedisReplyType::Error,

            str_val: value.to_owned(),
            int_val: 0,
            arr_val: Vec::new(),
        }
    }

    ///
    pub fn from_integer(value: i64) -> Self {
        Self {
            rpl_type: RedisReplyType::Integer,
            str_val: "".to_owned(),
            int_val: value,
            arr_val: Vec::new(),
        }
    }

    ///
    pub fn from_vec(value: Vec<RedisReply>) -> Self {
        Self {
            rpl_type: RedisReplyType::Array,
            str_val: "".to_owned(),
            int_val: 0,
            arr_val: value,
        }
    }

    ///
    #[inline(always)]
    pub fn reply_type(&self) -> RedisReplyType {
        self.rpl_type
    }

    ///
    pub fn error(&self) -> &str {
        assert!(self.is_error());
        &self.str_val
    }

    ///
    #[inline(always)]
    pub fn is_string(&self) -> bool {
        self.rpl_type == RedisReplyType::BulkString
            || self.rpl_type == RedisReplyType::SimpleString
            || self.rpl_type == RedisReplyType::Error
    }

    ///
    #[inline(always)]
    pub fn is_integer(&self) -> bool {
        self.rpl_type == RedisReplyType::Integer
    }

    ///
    #[inline(always)]
    pub fn is_array(&self) -> bool {
        self.rpl_type == RedisReplyType::Array
    }

    ///
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        self.rpl_type == RedisReplyType::Error
    }

    ///
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.rpl_type == RedisReplyType::Null
    }

    ///
    #[inline(always)]
    pub fn is_ok(&self) -> bool {
        !self.is_error()
    }

    ///
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        !self.is_error() && !self.is_null()
    }

    ///
    pub fn as_string(&self) -> &str {
        assert!(self.is_string());
        &self.str_val
    }

    ///
    pub fn as_integer(&self) -> i64 {
        assert!(self.is_integer());
        self.int_val
    }

    ///
    pub fn as_array(&self) -> &Vec<RedisReply> {
        assert!(self.is_array());
        &self.arr_val
    }
}
