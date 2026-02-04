use chrono::{DateTime, NaiveDate, Utc};

/// Canonical EU date format (day/month/year, zero padded).
pub const EU_DATE_FORMAT: &str = "%d/%m/%Y";
const EU_DATE_TIME_FORMAT: &str = "%d/%m/%Y %H:%M:%S";

/// Format a `NaiveDate` using the EU standard.
pub fn format_eu_date(date: NaiveDate) -> String {
    date.format(EU_DATE_FORMAT).to_string()
}

/// Format a `DateTime<Utc>` using day-first notation including time.
pub fn format_eu_date_time(dt: DateTime<Utc>) -> String {
    dt.format(EU_DATE_TIME_FORMAT).to_string()
}

/// Parse a user-supplied EU date string (DD/MM/YYYY) into `NaiveDate`.
pub fn parse_eu_date(candidate: &str) -> Option<NaiveDate> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(trimmed, EU_DATE_FORMAT).ok()
}

/// Serde helpers for Day/Month/Year formatting.
pub mod serde_eu_date {
    use super::EU_DATE_FORMAT;
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.format(EU_DATE_FORMAT).to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, EU_DATE_FORMAT).map_err(serde::de::Error::custom)
    }

    pub mod option {
        use super::EU_DATE_FORMAT;
        use chrono::NaiveDate;
        use serde::{Deserialize, Deserializer, Serializer};

        pub fn serialize<S>(value: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match value {
                Some(date) => super::serialize(date, serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let opt = Option::<String>::deserialize(deserializer)?;
            match opt {
                Some(value) => {
                    let date = NaiveDate::parse_from_str(&value, EU_DATE_FORMAT)
                        .map_err(serde::de::Error::custom)?;
                    Ok(Some(date))
                }
                None => Ok(None),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    #[test]
    fn formats_and_parses_day_first_dates() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 3).unwrap();
        let formatted = format_eu_date(date);
        assert_eq!(formatted, "03/02/2026");
        let parsed = parse_eu_date(&formatted).expect("parse");
        assert_eq!(parsed, date);
    }

    #[test]
    fn formats_date_time_with_clock() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 3, 15, 45, 10).unwrap();
        let formatted = format_eu_date_time(dt);
        assert_eq!(formatted, "03/02/2026 15:45:10");
    }
}
