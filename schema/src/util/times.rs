use chrono::{DateTime, Local, TimeZone, Utc};

pub fn current_time_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub trait ToTimeString {
    fn to_time_string(&self) -> String;
    fn to_time_string_shorter(&self) -> String;
    fn to_time_string_shorter_no_seconds(&self) -> String;
}

impl ToTimeString for i64 {
    fn to_time_string(&self) -> String {
        let utc_datetime = Utc.timestamp_millis(*self);
        let pacific_datetime: DateTime<Local> = utc_datetime.with_timezone(&Local);
        let formatted_datetime = pacific_datetime.format("%Y-%m-%d %H:%M:%S %.3f %:z").to_string();
        return formatted_datetime;
    }

    fn to_time_string_shorter(&self) -> String {
        let utc_datetime = Utc.timestamp_millis(*self);
        let pacific_datetime: DateTime<Local> = utc_datetime.with_timezone(&Local);
        let formatted_datetime = pacific_datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        return formatted_datetime;
    }

    fn to_time_string_shorter_no_seconds(&self) -> String {
        let utc_datetime = Utc.timestamp_millis(*self);
        let pacific_datetime: DateTime<Local> = utc_datetime.with_timezone(&Local);
        let formatted_datetime = pacific_datetime.format("%Y-%m-%d %H:%M").to_string();
        return formatted_datetime;
    }

}