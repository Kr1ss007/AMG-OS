//! Clock & Calendar Tray Widget
//!
//! Provides the primary system time display in the Top Global Menu Panel.
//! Formats localized time (24h/12h), provides full date tooltip,
//! and toggles an interactive monthly calendar popover.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockTrayWidget {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub weekday: String,
    pub is_24h_format: bool,
    pub calendar_visible: bool,
}

impl ClockTrayWidget {
    pub fn new() -> Self {
        Self {
            hours: 9,
            minutes: 45,
            seconds: 0,
            year: 2026,
            month: 9,
            day: 15,
            weekday: "Tue".to_string(),
            is_24h_format: true,
            calendar_visible: false,
        }
    }

    /// Update current clock time
    pub fn update_time(
        &mut self,
        hours: u8,
        minutes: u8,
        seconds: u8,
        year: i32,
        month: u8,
        day: u8,
        weekday: &str,
    ) {
        self.hours = hours;
        self.minutes = minutes;
        self.seconds = seconds;
        self.year = year;
        self.month = month;
        self.day = day;
        self.weekday = weekday.to_string();
    }

    /// Primary display string rendered in the Top Panel (e.g. "Tue Sep 15  09:45")
    pub fn display_string(&self) -> String {
        let time_str = self.formatted_time();
        let month_str = match self.month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            _ => "Dec",
        };
        format!("{} {} {}  {}", self.weekday, month_str, self.day, time_str)
    }

    /// Formatted time according to 24h or 12h preferences
    pub fn formatted_time(&self) -> String {
        if self.is_24h_format {
            format!("{:02}:{:02}", self.hours, self.minutes)
        } else {
            let h = if self.hours == 0 {
                12
            } else if self.hours > 12 {
                self.hours - 12
            } else {
                self.hours
            };
            let period = if self.hours >= 12 { "PM" } else { "AM" };
            format!("{:02}:{:02} {}", h, self.minutes, period)
        }
    }

    /// Days in the currently displayed month
    pub fn days_in_month(&self) -> u8 {
        match self.month {
            4 | 6 | 9 | 11 => 30,
            2 => {
                let is_leap = (self.year % 4 == 0 && self.year % 100 != 0) || (self.year % 400 == 0);
                if is_leap {
                    29
                } else {
                    28
                }
            }
            _ => 31,
        }
    }

    pub fn toggle_calendar(&mut self) {
        self.calendar_visible = !self.calendar_visible;
    }
}

impl Default for ClockTrayWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_formatting_and_calendar() {
        let mut clock = ClockTrayWidget::new();
        clock.update_time(14, 5, 0, 2026, 9, 15, "Tue");

        assert_eq!(clock.formatted_time(), "14:05");
        assert_eq!(clock.display_string(), "Tue Sep 15  14:05");
        assert_eq!(clock.days_in_month(), 30);

        clock.is_24h_format = false;
        assert_eq!(clock.formatted_time(), "02:05 PM");
    }
}
