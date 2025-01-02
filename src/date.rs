use chrono::Local;
use chrono::NaiveDate;

pub fn parse_date(date_str: &str) -> Result<NaiveDate, String> {
    // Handle special keywords
    match date_str.to_lowercase().as_str() {
        "today" => return Ok(Local::now().date_naive()),
        "yesterday" => {
            return Ok(Local::now()
                .date_naive()
                .pred_opt()
                .ok_or("Failed to calculate yesterday's date")?);
        }
        _ => {}
    }

    // Try YYYY-MM-DD format
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Ok(date);
    }

    // Try YYYYMMDD format
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y%m%d") {
        return Ok(date);
    }

    Err("Invalid date format. Please use YYYY-MM-DD, YYYYMMDD, 'today', or 'yesterday'".to_string())
}

/// Calculates number of days between two dates, inclusive of both dates
pub fn days_between(from: NaiveDate, to: NaiveDate) -> i64 {
    (to - from).num_days()
}

/// Calculates the millisecond offset for the SQL query based on number of days
pub fn calculate_id_offset(days: i64) -> i64 {
    days * 86_400_000 // milliseconds per day
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_days_between() {
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        assert_eq!(days_between(from, to), 2); // Jan 1 to Jan 3 = 2 days

        // Same date should return 0 days
        assert_eq!(days_between(from, from), 0);

        // Jan 1 to Jan 2 should be 1 day
        let jan2 = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        assert_eq!(days_between(from, jan2), 1);
    }

    #[test]
    fn test_calculate_id_offset() {
        assert_eq!(calculate_id_offset(1), 86_400_000);
        assert_eq!(calculate_id_offset(2), 86_400_000 * 2);
        assert_eq!(calculate_id_offset(7), 86_400_000 * 7);
    }

    #[test]
    fn test_parse_date_special_keywords() {
        let today = Local::now().date_naive();
        let yesterday = today.pred();

        assert_eq!(parse_date("today").unwrap(), today);
        assert_eq!(parse_date("TODAY").unwrap(), today);
        assert_eq!(parse_date("yesterday").unwrap(), yesterday);
        assert_eq!(parse_date("YESTERDAY").unwrap(), yesterday);
    }

    #[test]
    fn test_parse_date_formats() {
        let expected = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(parse_date("2024-01-15").unwrap(), expected);
        assert_eq!(parse_date("20240115").unwrap(), expected);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2024-13-45").is_err());
        assert!(parse_date("20241345").is_err());
    }
}