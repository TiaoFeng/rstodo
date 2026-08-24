//! 时间处理模块
//!
//! 用于处理deadline转UTC时间保存，
//! 保存的deadline转为当地时间输出，
//! 解析用户传入的包含时间的字符串
use crate::error::AppError;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// 将用户输入的当地时间，转换成UTC时间用于保存
pub fn to_utc(native: &NaiveDateTime) -> Result<DateTime<Utc>, AppError> {
    let local = Local
        .from_local_datetime(native)
        .single()
        .ok_or(AppError::InvalidLocalTime)?;
    Ok(local.with_timezone(&Utc))
}

/// 将保存的UTC时间转换成当地时间用于输出
pub fn to_local_time(utc_time: &DateTime<Utc>) -> NaiveDateTime {
    let local = utc_time.with_timezone(&Local);
    local.naive_local()
}

/// 将用户输入的含有时间的字符串切片转换成标准的DateTime格式
///
/// 逻辑：
/// 1. 用户输入`%Y-%m-%d`格式的时间，自动在末尾加上`23:59:59`
/// 2. 用户输入`%Y-%m-%dT%H:%M:%S`，提取时间
/// 3. 使用to_utc转换成UTC时间返回
pub fn parse_deadline_input(input: &str) -> Result<DateTime<Utc>, AppError> {
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        let datetime = date.and_hms_opt(23, 59, 59).unwrap();
        return to_utc(&datetime);
    }
    let datetime = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S").map_err(|_| {
        AppError::InvalidDeadline {
            input: input.to_string(),
        }
    })?;
    to_utc(&datetime)
}
