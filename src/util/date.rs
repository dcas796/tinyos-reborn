use core::fmt;
use core::fmt::Formatter;

#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Date {
    unix_seconds: u64,
}

impl Date {
    pub fn from_unix_seconds(unix_seconds: u64) -> Self {
        Self { unix_seconds }
    }

    pub fn from_date_time(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Option<Self> {
        if day < 1 || hour > 23 || minute > 59 || second > 59 {
            return None;
        }

        if day > Self::days_in_month(year, month)? {
            return None;
        }

        let mut total_days = (year as u64).checked_sub(1970)? * 365;
        for y in 1970..year {
            if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                total_days += 1;
            }
        }

        for m in 1..month {
            total_days += Self::days_in_month(year, m)? as u64;
        }

        total_days += (day - 1) as u64;

        let total_seconds = total_days * 24 * 60 * 60
            + (hour as u64) * 60 * 60
            + (minute as u64) * 60
            + (second as u64);

        Some(Self::from_unix_seconds(total_seconds))
    }

    fn is_leap_year(year: u16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    fn days_in_month(year: u16, month: u8) -> Option<u8> {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
            4 | 6 | 9 | 11 => Some(30),
            2 => {
                if Self::is_leap_year(year) {
                    Some(29)
                } else {
                    Some(28)
                }
            }
            _ => None,
        }
    }

    fn days_in_year(year: u16) -> u16 {
        if Self::is_leap_year(year) {
            366
        } else {
            365
        }
    }
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Date({})", self.unix_seconds)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut seconds = self.unix_seconds;
        let days = seconds / (24 * 60 * 60);
        seconds %= 24 * 60 * 60;
        let hours = seconds / (60 * 60);
        seconds %= 60 * 60;
        let minutes = seconds / 60;
        seconds %= 60;

        let mut year = 1970;
        let mut day_of_year = days as u16;

        while day_of_year >= Self::days_in_year(year) {
            day_of_year -= Self::days_in_year(year);
            year += 1;
        }

        let mut month = 1;
        while day_of_year >= Self::days_in_month(year, month).unwrap() as u16 {
            day_of_year -= Self::days_in_month(year, month).unwrap() as u16;
            month += 1;
        }

        let day = day_of_year + 1;

        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            year, month, day, hours, minutes, seconds
        )
    }
}
