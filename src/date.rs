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

pub fn validate_dates(from_date: Option<NaiveDate>, to_date: Option<NaiveDate>, today: NaiveDate) -> Result<(), String> {
    // println!("Validating dates...");
    // println!("from_date: {:?}", from_date);
    // println!("to_date: {:?}", to_date);
    // println!("today: {:?}", today);

    // Ensure 'to_date' is not in the future
    if let Some(to) = to_date {
        // println!("Checking if 'to_date' ({}) is in the future...", to);
        if to > today {
            return Err(format!("Invalid 'to_date': {} is in the future.", to));
        }
    }

    // Ensure 'from_date' is not in the future
    if let Some(from) = from_date {
        println!("Checking if 'from_date' ({}) is in the future...", from);
        if from > today {
            return Err(format!("Invalid 'from_date': {} is in the future.", from));
        }
    }

    // Check for invalid date range
    if let (Some(from), Some(to)) = (from_date, to_date) {
        // println!("Checking date range: from_date ({}) > to_date ({})", from, to);
        if from <= to {
            return Err(format!(
                "Invalid date range: 'from_date' ({}) must be after 'to_date' ({}).",
                from, to
            ));
        }
    }

    println!("Dates are valid.");
    Ok(())
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
        let yesterday = today.pred_opt().unwrap();

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

    fn mock_today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 1, 4).unwrap() // Mocked "today" for testing
    }

    #[test]
    fn test_validate_dates_valid_case() {
        let from_date = Some(NaiveDate::from_ymd_opt(2025, 1, 3).unwrap()); // Not in the future
        let to_date = Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()); // Earlier than from_date
        let today = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap(); // Mocked current date

        let result = validate_dates(from_date, to_date, today);

        assert!(result.is_ok(), "Expected Ok(()), got: {:?}", result);
    }

    #[test]
    fn test_validate_dates_valid_from_only() {
        let from_date = Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        let today = mock_today();
        assert!(validate_dates(from_date, None, today).is_ok());
    }

    #[test]
    fn test_validate_dates_invalid_date_order() {
        let from_date = Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        let to_date = Some(NaiveDate::from_ymd_opt(2025, 1, 5).unwrap());
        let today = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();

        let result = validate_dates(from_date, to_date, today);

        let expected_errors = vec![
            "Invalid date range: 'from_date' (2025-01-01) must be after 'to_date' (2025-01-05).".to_string(),
            "Invalid 'to_date': 2025-01-05 is in the future.".to_string(),
        ];

        assert!(
            expected_errors.contains(&result.clone().unwrap_err()),
            "Unexpected error: {:?}",
            result
        );
    }



}