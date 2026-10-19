//! The agent log: one line per failed proofread, with its time, its cause, the model and the
//! duration. Never the selection, the correction or the API key.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::ProofreadError;

/// A text file the agent appends its failures to.
#[derive(Debug, Clone)]
pub(crate) struct AgentLog {
    path: PathBuf,
}

impl AgentLog {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Appends the line of a failed proofread, creating the file and its folder if needed.
    pub(crate) fn failure(&self, cause: &ProofreadError, model: &str, duration: Duration) -> io::Result<()> {
        let line = format!("{}  {cause}  model={model}  duration={}ms", timestamp(SystemTime::now()), duration.as_millis());
        append(&self.path, &line)
    }
}

fn append(path: &Path, line: &str) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

/// The time in UTC, `YYYY-MM-DD HH:MM:SSZ`. A clock before 1970 reads as 1970.
fn timestamp(now: SystemTime) -> String {
    let seconds = now.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let (days, of_day) = (seconds / 86_400, seconds % 86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z", of_day / 3600, of_day % 3600 / 60, of_day % 60)
}

/// The calendar date of a count of days since 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days.saturating_add(719_468);
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The seams cannot fix the clock: the date arithmetic is checked here, on known dates.
    #[test]
    fn the_timestamp_is_the_utc_date_and_time() {
        let at = |seconds| timestamp(UNIX_EPOCH + Duration::from_secs(seconds));

        assert_eq!(at(0), "1970-01-01 00:00:00Z");
        assert_eq!(at(951_782_400), "2000-02-29 00:00:00Z");
        assert_eq!(at(1_792_310_405), "2026-10-18 08:00:05Z");
    }
}
