use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

pub fn to_utc(native: &NaiveDateTime) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let local = Local
        .from_local_datetime(native)
        .single()
        .ok_or("Time Conversion Error")?;
    Ok(local.with_timezone(&Utc))
}

pub fn to_local_time(
    utc_time: &DateTime<Utc>,
) -> Result<NaiveDateTime, Box<dyn std::error::Error>> {
    let local = utc_time.with_timezone(&Local);
    Ok(local.naive_local())
}

pub fn parse_deadline_input(input: &str) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        let datetime = date.and_hms_opt(0, 0, 0).unwrap();
        return to_utc(&datetime);
    }
    let datetime = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S")?;
    to_utc(&datetime)
}
