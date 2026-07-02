use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use tracing::debug;

use crate::session::db::Schedule;

const HEADER: &str = "# Tasks\n\n";

pub fn load(path: &Path) -> Result<Vec<Schedule>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read schedule file: {}", path.display()))?;

    let mut schedules = Vec::new();
    let mut id_counter: i64 = 1;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(schedule) = parse_line(line, id_counter) {
            schedules.push(schedule);
            id_counter += 1;
        }
    }

    debug!("Loaded {} schedules from {}", schedules.len(), path.display());
    Ok(schedules)
}

pub fn save(path: &Path, schedules: &[Schedule]) -> Result<()> {
    let mut content = String::from(HEADER);

    for schedule in schedules {
        content.push_str(&format_line(schedule));
        content.push('\n');
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, &content)
        .with_context(|| format!("Failed to write schedule file: {}", path.display()))?;

    debug!("Saved {} schedules to {}", schedules.len(), path.display());
    Ok(())
}

fn parse_line(line: &str, id: i64) -> Option<Schedule> {
    let line = line.trim();
    if !line.starts_with("- [") {
        return None;
    }

    let completed = line.starts_with("- [x]");
    let rest = if completed {
        line.strip_prefix("- [x]")?.trim_start()
    } else if line.starts_with("- [ ]") {
        line.strip_prefix("- [ ]")?.trim_start()
    } else {
        return None;
    };

    let status = if completed {
        "completed"
    } else if rest.starts_with('\u{23F8}') {
        "paused"
    } else {
        "active"
    };

    let mut message = String::new();
    let mut next_run_at: Option<i64> = None;
    let mut interval_seconds: Option<i64> = None;

    let mut remaining = rest.to_string();

    while !remaining.is_empty() {
        if let Some(pos) = remaining.find('\u{1F4C5}') {
            message = remaining[..pos].trim().to_string();
            remaining = remaining[pos..].trim_start().to_string();

            remaining = remaining.strip_prefix('\u{1F4C5}').unwrap_or(&remaining).trim_start().to_string();

            let date_end = remaining.find(|c: char| c == ' ' || c == '\u{1F501}' || c == '\u{2705}').unwrap_or(remaining.len());
            let date_str = remaining[..date_end].to_string();

            if let Ok(dt) = DateTime::parse_from_rfc3339(&date_str) {
                next_run_at = Some(dt.timestamp());
            } else if let Ok(dt) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                next_run_at = Some(dt.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp());
            } else if let Some(dt) = try_parse_datetime(&date_str) {
                next_run_at = Some(dt.timestamp());
            }

            remaining = remaining[date_end..].trim_start().to_string();
        } else if let Some(pos) = remaining.find('\u{1F501}') {
            if message.is_empty() {
                message = remaining[..pos].trim().to_string();
            }
            remaining = remaining[pos..].trim_start().to_string();

            remaining = remaining.strip_prefix('\u{1F501}').unwrap_or(&remaining).trim_start().to_string();
            let num_end = remaining.find(|c: char| c == ' ').unwrap_or(remaining.len());
            let num_str = remaining[..num_end].to_string();
            if let Ok(n) = num_str.parse::<i64>() {
                interval_seconds = Some(n);
            }
            remaining = remaining[num_end..].trim_start().to_string();
        } else if let Some(pos) = remaining.find('\u{2705}') {
            if message.is_empty() {
                message = remaining[..pos].trim().to_string();
            }
            remaining = remaining[pos..].trim_start().to_string();
            remaining = remaining.strip_prefix('\u{2705}').unwrap_or(&remaining).trim_start().to_string();

            let date_end = remaining.find(' ').unwrap_or(remaining.len());
            remaining = remaining[date_end..].trim_start().to_string();
        } else {
            if message.is_empty() {
                message = remaining.trim().to_string();
            }
            break;
        }
    }

    message = message
        .trim_start_matches('\u{23F8}')
        .trim_start_matches('\u{1F3C3}')
        .trim()
        .to_string();

    if message.is_empty() {
        return None;
    }

    Some(Schedule {
        id,
        message,
        next_run_at: next_run_at.unwrap_or(0),
        interval_seconds,
        status: status.to_string(),
        created_at: 0,
    })
}

fn try_parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

fn format_line(schedule: &Schedule) -> String {
    let dt = DateTime::from_timestamp(schedule.next_run_at, 0)
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let checkbox = match schedule.status.as_str() {
        "completed" => "- [x]",
        _ => "- [ ]",
    };

    let status_prefix = match schedule.status.as_str() {
        "paused" => '\u{23F8}'.to_string(),
        _ => String::new(),
    };

    let mut line = format!("{} {}{}", checkbox, status_prefix, schedule.message);

    line.push_str(&format!(" \u{1F4C5}{}", dt));

    if let Some(interval) = schedule.interval_seconds {
        line.push_str(&format!(" \u{1F501}{}", interval));
    }

    if schedule.status == "completed" {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        line.push_str(&format!(" \u{2705}{}", now));
    }

    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::db::Schedule;

    #[test]
    fn test_roundtrip() {
        let schedules = vec![
            Schedule {
                id: 1,
                message: "Buy groceries".to_string(),
                next_run_at: 1720000000,
                interval_seconds: Some(86400),
                status: "active".to_string(),
                created_at: 0,
            },
            Schedule {
                id: 2,
                message: "Submit report".to_string(),
                next_run_at: 1720000000,
                interval_seconds: None,
                status: "completed".to_string(),
                created_at: 0,
            },
            Schedule {
                id: 3,
                message: "Review code".to_string(),
                next_run_at: 1720000000,
                interval_seconds: Some(3600),
                status: "paused".to_string(),
                created_at: 0,
            },
        ];

        let md = schedules.iter().map(format_line).collect::<Vec<_>>();
        let parsed: Vec<Schedule> = md.iter().map(|s| parse_line(s, 1).unwrap()).collect();

        assert_eq!(parsed[0].message, "Buy groceries");
        assert_eq!(parsed[0].status, "active");
        assert_eq!(parsed[0].interval_seconds, Some(86400));

        assert_eq!(parsed[1].message, "Submit report");
        assert_eq!(parsed[1].status, "completed");

        assert_eq!(parsed[2].message, "Review code");
        assert_eq!(parsed[2].status, "paused");
        assert_eq!(parsed[2].interval_seconds, Some(3600));
    }
}