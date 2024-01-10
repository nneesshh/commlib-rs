use chrono::{DateTime, TimeZone, Timelike, Utc};
use mysql::Value as MySqlVal;
use serde_json::{Number, Value as Json};

static EMPTY_VEC_U8: Vec<u8> = Vec::new();

///
#[allow(dead_code)]
#[inline(always)]
pub fn to_json(val: &MySqlVal) -> Json {
    //
    value_to_json(val)
}

///
#[allow(dead_code)]
#[inline(always)]
pub fn to_string(val: &MySqlVal) -> String {
    //
    value_to_string(val)
}

///
#[inline(always)]
pub fn value_to_json(val: &MySqlVal) -> Json {
    match val {
        MySqlVal::NULL => {
            //
            Json::Null
        }
        MySqlVal::Bytes(bytes) => {
            //
            Json::String(unsafe { std::str::from_utf8_unchecked(bytes.as_slice()).to_owned() })
        }
        MySqlVal::Int(n) => {
            //
            Json::Number(Number::from(*n))
        }
        MySqlVal::UInt(n) => {
            //
            Json::Number(Number::from(*n))
        }
        MySqlVal::Float(f) => {
            //
            Json::Number(Number::from_f64(*f as f64).unwrap())
        }
        MySqlVal::Double(f) => {
            //
            Json::Number(Number::from_f64(*f).unwrap())
        }
        MySqlVal::Date(0, 0, 0, 0, 0, 0, 0) => {
            //
            let dt: DateTime<Utc> = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
            Json::String(dt.to_rfc3339())
        }
        MySqlVal::Date(year, month, day, hour, minutes, seconds, micro_seconds) => {
            //
            let dt: DateTime<Utc> = Utc
                .with_ymd_and_hms(
                    *year as i32,
                    *month as u32,
                    *day as u32,
                    *hour as u32,
                    *minutes as u32,
                    *seconds as u32,
                )
                .unwrap();
            if *micro_seconds > 0 {
                dt.with_nanosecond((*micro_seconds) * 1000);
            }
            Json::String(dt.to_rfc3339())
        }
        MySqlVal::Time(is_negative, days, hours, minutes, seconds, micro_seconds) => {
            // HH:MM:SS
            let total_hours = (*days) * 24 + (*hours) as u32;
            let total_nanos = (*micro_seconds) * 1000;
            let sign = if *is_negative { "-" } else { "" };
            if total_nanos > 0 {
                Json::String(std::format!(
                    "{}{}:{:02}:{:02}.{:09}",
                    sign,
                    total_hours,
                    minutes,
                    seconds,
                    total_nanos
                ))
            } else {
                Json::String(std::format!(
                    "{}{}:{:02}:{:02}",
                    sign,
                    total_hours,
                    minutes,
                    seconds
                ))
            }
        }
    }
}

///
#[inline(always)]
pub fn value_to_string(val: &MySqlVal) -> String {
    match val {
        MySqlVal::NULL => {
            //
            "".to_owned()
        }
        MySqlVal::Bytes(bytes) => unsafe {
            //
            std::str::from_utf8_unchecked(bytes.as_slice()).to_owned()
        },
        MySqlVal::Int(n) => {
            //
            (*n).to_string()
        }
        MySqlVal::UInt(n) => {
            //
            (*n).to_string()
        }
        MySqlVal::Float(f) => {
            //
            (*f).to_string()
        }
        MySqlVal::Double(f) => {
            //
            (*f).to_string()
        }
        MySqlVal::Date(0, 0, 0, 0, 0, 0, 0) => {
            //
            let dt: DateTime<Utc> = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
            dt.to_rfc3339()
        }
        MySqlVal::Date(year, month, day, hour, minutes, seconds, micro_seconds) => {
            //
            let dt: DateTime<Utc> = Utc
                .with_ymd_and_hms(
                    *year as i32,
                    *month as u32,
                    *day as u32,
                    *hour as u32,
                    *minutes as u32,
                    *seconds as u32,
                )
                .unwrap();
            if *micro_seconds > 0 {
                dt.with_nanosecond((*micro_seconds) * 1000);
            }
            dt.to_rfc3339()
        }
        MySqlVal::Time(is_negative, days, hours, minutes, seconds, micro_seconds) => {
            //
            // HH:MM:SS
            let total_hours = (*days) * 24 + (*hours) as u32;
            let total_nanos = (*micro_seconds) * 1000;
            let sign = if *is_negative { "-" } else { "" };
            if total_nanos > 0 {
                std::format!(
                    "{}{}:{:02}:{:02}.{:09}",
                    sign,
                    total_hours,
                    minutes,
                    seconds,
                    total_nanos
                )
            } else {
                std::format!("{}{}:{:02}:{:02}", sign, total_hours, minutes, seconds)
            }
        }
    }
}

///
#[inline(always)]
pub fn value_to_str(val: &MySqlVal) -> Option<&str> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Bytes(bytes) => {
            //
            Some(unsafe { std::str::from_utf8_unchecked(bytes.as_slice()) })
        }
        _ => Some(""),
    }
}

///
#[inline(always)]
pub fn value_to_blob(val: &MySqlVal) -> Option<&Vec<u8>> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Bytes(bytes) => {
            //
            Some(&bytes)
        }
        _ => Some(&EMPTY_VEC_U8),
    }
}

///
#[inline(always)]
pub fn value_to_int64(val: &MySqlVal) -> Option<i64> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Int(n) => Some(*n),
        _ => Some(0),
    }
}

///
#[inline(always)]
pub fn value_to_uint64(val: &MySqlVal) -> Option<u64> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Int(n) => Some(*n as u64),
        _ => Some(0),
    }
}

///
#[inline(always)]
pub fn value_to_float(val: &MySqlVal) -> Option<f32> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Float(f) => Some(*f),
        _ => Some(0_f32),
    }
}

///
#[inline(always)]
pub fn value_to_double(val: &MySqlVal) -> Option<f64> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Double(f) => Some(*f),
        _ => Some(0_f64),
    }
}

///
#[inline(always)]
pub fn value_to_timestamp(val: &MySqlVal) -> Option<(i64, u32)> {
    let ret = value_to_date(val);
    match ret {
        Some((0, 0, 0, 0, 0, 0, 0)) => {
            //
            Some((0, 0))
        }
        Some((year, month, day, hour, minutes, seconds, micro_seconds)) => {
            //
            let dt: DateTime<Utc> = Utc
                .with_ymd_and_hms(
                    year as i32,
                    month as u32,
                    day as u32,
                    hour as u32,
                    minutes as u32,
                    seconds as u32,
                )
                .unwrap();
            Some((dt.timestamp(), micro_seconds))
        }
        None => None,
    }
}

///
#[inline(always)]
pub fn value_to_date(val: &MySqlVal) -> Option<(u16, u8, u8, u8, u8, u8, u32)> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Date(year, month, day, hour, minutes, seconds, micro_seconds) => {
            //
            Some((
                *year,
                *month,
                *day,
                *hour,
                *minutes,
                *seconds,
                *micro_seconds,
            ))
        }
        _ => Some((0, 0, 0, 0, 0, 0, 0)),
    }
}

///
#[inline(always)]
pub fn value_to_time(val: &MySqlVal) -> Option<(bool, u32, u8, u8, u8, u32)> {
    match val {
        MySqlVal::NULL => None,
        MySqlVal::Time(is_negative, days, hours, minutes, seconds, micro_seconds) => {
            //
            Some((
                *is_negative,
                *days,
                *hours,
                *minutes,
                *seconds,
                *micro_seconds,
            ))
        }
        _ => Some((false, 0, 0, 0, 0, 0)),
    }
}
