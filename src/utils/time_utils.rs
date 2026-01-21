use chrono::{DateTime, Local, TimeZone};

// Very specific date format the old API is doing: dd/MM/yyyy HH:mm:ssZ
// chrono formatting reference:
// https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html
const DATE_FORMAT_STANDARD: &'static str = "%d/%m/%Y %k:%M:%S%:z";
const DATE_FORMAT_USCOMPACT: &'static str = "%Y-%m-%d";

pub enum DateFormat {
    Standard,
    USCompact,
}

pub fn timestamp_to_date_string(timestamp: i64, format: DateFormat) -> String {
    let d = Local.timestamp(timestamp, 0);
    let format_str = match format {
        DateFormat::Standard => DATE_FORMAT_STANDARD,
        DateFormat::USCompact => DATE_FORMAT_USCOMPACT,
    };
    d.format(format_str).to_string()
}

pub fn current_timestamp() -> i64 {
    Local::now().timestamp()
}

pub fn current_datetime_rfc2822() -> String {
    Local::now().to_rfc2822()
}

pub fn parse_nginx_log_date(s: &str) -> Option<DateTime<Local>> {
    parse_date_with_format(s, "%d/%b/%Y:%H:%M:%S %z")
}

fn parse_date_with_format(s: &str, format: &str) -> Option<DateTime<Local>> {
    match DateTime::parse_from_str(s, format) {
        Ok(date) => Some(date.into()),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_time_formats_as_expected() {
        let timestamp: i64 = 1615150740;
        let result = timestamp_to_date_string(timestamp, DateFormat::Standard);
        assert_eq!("07/03/2021 21:59:00+01:00", result);
    }

    #[test]
    fn can_parse_nginx_log_date() {
        let sut = "20/Jan/2026:09:30:01 +0100";
        // I'm not super sure that's the expected result
        let expected = "2026-01-20 09:30:01";
        let parsed = parse_nginx_log_date(sut);
        let formatted = parsed.unwrap().format("%Y-%m-%d %H:%M:%S").to_string();
        assert_eq!(expected, formatted)
    }
}
