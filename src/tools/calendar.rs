use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::debug;

pub struct CalendarManager {
    browser_bin: PathBuf,
    reminder_bin: Option<PathBuf>,
    thunderbird_bin: Option<PathBuf>,
    events_dir: PathBuf,
}

impl Default for CalendarManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CalendarManager {
    pub fn new() -> Self {
        let browser_bin = which::which("omarchy-launch-browser")
            .or_else(|_| which::which("xdg-open"))
            .unwrap_or_else(|_| PathBuf::from("xdg-open"));

        let reminder_bin = which::which("omarchy-reminder").ok();
        let thunderbird_bin = which::which("thunderbird").ok();

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let events_dir = PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("jarvis")
            .join("events");
        let _ = fs::create_dir_all(&events_dir);

        Self {
            browser_bin,
            reminder_bin,
            thunderbird_bin,
            events_dir,
        }
    }

    pub fn parse_event_datetime(time_str: &str) -> NaiveDateTime {
        let now = Local::now().naive_local();
        let mut clean = time_str.trim().to_lowercase();

        // Check standard date formats
        for fmt in &["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
            if let Ok(dt) = NaiveDateTime::parse_from_str(&clean, fmt) {
                return dt;
            }
        }
        if let Ok(d) = NaiveDate::parse_from_str(&clean, "%Y-%m-%d") {
            return d.and_hms_opt(9, 0, 0).unwrap_or(now);
        }

        let mut day_offset = 0;
        if clean.contains("day after tomorrow") {
            day_offset = 2;
            clean = clean.replace("day after tomorrow", "").trim().to_string();
        } else if clean.contains("tomorrow") {
            day_offset = 1;
            clean = clean.replace("tomorrow", "").trim().to_string();
        } else if clean.contains("today") {
            day_offset = 0;
            clean = clean.replace("today", "").trim().to_string();
        } else {
            let weekdays = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
            for (idx, day) in weekdays.iter().enumerate() {
                if clean.contains(day) {
                    let current_weekday = now.weekday().num_days_from_monday() as usize;
                    let days_ahead = (idx + 7 - current_weekday) % 7;
                    day_offset = if days_ahead == 0 { 7 } else { days_ahead as i64 };
                    clean = clean.replace(day, "").replace("next", "").trim().to_string();
                    break;
                }
            }
        }

        let target_date = now.date() + Duration::days(day_offset);
        clean = clean.replace("at", "").trim().to_string();

        // 12-hour format: 3pm, 3:30 pm, 11am
        let re_12h = Regex::new(r"(\d{1,2})(?::(\d{2}))?\s*(am|pm)").unwrap();
        if let Some(cap) = re_12h.captures(&clean) {
            let mut hour: u32 = cap[1].parse().unwrap_or(9);
            let minute: u32 = cap.get(2).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
            let meridiem = &cap[3];
            if meridiem == "pm" && hour != 12 {
                hour += 12;
            } else if meridiem == "am" && hour == 12 {
                hour = 0;
            }
            if let Some(t) = NaiveTime::from_hms_opt(hour, minute, 0) {
                return NaiveDateTime::new(target_date, t);
            }
        }

        // 24-hour format: 14:30
        let re_24h = Regex::new(r"(\d{1,2}):(\d{2})").unwrap();
        if let Some(cap) = re_24h.captures(&clean) {
            let hour: u32 = cap[1].parse().unwrap_or(9);
            let minute: u32 = cap[2].parse().unwrap_or(0);
            if let Some(t) = NaiveTime::from_hms_opt(hour, minute, 0) {
                return NaiveDateTime::new(target_date, t);
            }
        }

        // Single number: 3 or 9
        let re_num = Regex::new(r"\b(\d{1,2})\b").unwrap();
        if let Some(cap) = re_num.captures(&clean) {
            let mut hour: u32 = cap[1].parse().unwrap_or(9);
            if hour < 8 {
                hour += 12;
            }
            if let Some(t) = NaiveTime::from_hms_opt(hour, 0, 0) {
                return NaiveDateTime::new(target_date, t);
            }
        }

        // Default: 9:00 AM on target date
        target_date.and_hms_opt(9, 0, 0).unwrap_or(now)
    }

    pub async fn schedule_event(
        &self,
        title: &str,
        start_time: &str,
        end_time: Option<&str>,
        description: &str,
        provider: &str,
    ) -> Result<String> {
        let dt_start = Self::parse_event_datetime(start_time);
        let dt_end = if let Some(e) = end_time {
            Self::parse_event_datetime(e)
        } else {
            dt_start + Duration::hours(1)
        };

        let start_str = dt_start.format("%Y%m%dT%H%M%S").to_string();
        let end_str = dt_end.format("%Y%m%dT%H%M%S").to_string();
        let nice_start = dt_start.format("%A, %B %d at %I:%M %p").to_string();

        // 1. Generate Google Calendar Template URL
        let encoded_title = urlencoding::encode(title);
        let encoded_desc = urlencoding::encode(description);
        let gcal_url = format!(
            "https://calendar.google.com/calendar/render?action=TEMPLATE&text={encoded_title}&details={encoded_desc}&dates={start_str}/{end_str}"
        );

        // 2. Generate RFC 5545 .ics file
        let clean_title_slug: String = title
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .take(20)
            .collect();
        let ics_filename = format!("{}_{clean_title_slug}.ics", dt_start.format("%Y%m%d_%H%M"));
        let ics_path = self.events_dir.join(ics_filename);

        let ics_content = format!(
            "BEGIN:VCALENDAR\n\
            VERSION:2.0\n\
            PRODID:-//Jarvis Assistant//EN\n\
            BEGIN:VEVENT\n\
            UID:{}@jarvis\n\
            DTSTAMP:{}\n\
            DTSTART:{start_str}\n\
            DTEND:{end_str}\n\
            SUMMARY:{title}\n\
            DESCRIPTION:{description}\n\
            STATUS:CONFIRMED\n\
            END:VEVENT\n\
            END:VCALENDAR\n",
            dt_start.and_utc().timestamp(),
            Local::now().format("%Y%m%dT%H%M%S")
        );
        let _ = fs::write(&ics_path, ics_content);

        debug!("Scheduling event '{title}' for {nice_start}");

        if (provider.eq_ignore_ascii_case("thunderbird") || provider.eq_ignore_ascii_case("local"))
            && self.thunderbird_bin.is_some()
        {
            if let Some(ref tb) = self.thunderbird_bin {
                let _ = Command::new(tb)
                    .arg("-file")
                    .arg(&ics_path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                return Ok(format!("Scheduled '{title}' for {nice_start} and opened in Thunderbird."));
            }
        }

        // Open Google Calendar with all fields prefilled
        let _ = Command::new(&self.browser_bin)
            .arg(&gcal_url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        Ok(format!("Scheduled '{title}' for {nice_start}. Opened Google Calendar for confirmation, sir."))
    }

    pub async fn set_reminder(&self, minutes: i64, message: &str) -> Result<String> {
        let mins = if minutes <= 0 { 5 } else { minutes };

        if let Some(ref rem_bin) = self.reminder_bin {
            let output = Command::new(rem_bin)
                .arg(mins.to_string())
                .arg(message)
                .output()
                .await?;

            let remind_time = (Local::now() + Duration::minutes(mins)).format("%I:%M %p").to_string();
            if output.status.success() {
                Ok(format!("Reminder set for {mins} minutes from now (at {remind_time}): '{message}', sir."))
            } else {
                let err = String::from_utf8_lossy(&output.stderr);
                Ok(format!("Failed to set reminder: {err}"))
            }
        } else {
            Ok("Reminder system not available on this system.".to_string())
        }
    }

    pub async fn list_reminders(&self) -> Result<String> {
        if let Some(ref rem_bin) = self.reminder_bin {
            let output = Command::new(rem_bin)
                .arg("show")
                .arg("--json")
                .output()
                .await?;

            if let Ok(data) = serde_json::from_slice::<Value>(&output.stdout) {
                if let Some(reminders) = data["reminders"].as_array() {
                    if reminders.is_empty() {
                        return Ok("You have no outstanding reminders, sir.".to_string());
                    }
                    let mut lines = vec!["Upcoming reminders:".to_string()];
                    for r in reminders {
                        let label = r["label"].as_str().unwrap_or("Reminder");
                        let remaining = r["remaining"].as_str().unwrap_or("");
                        let at_time = r["atTime"].as_str().unwrap_or("");
                        lines.push(format!("- '{label}' at {at_time} (in {remaining})"));
                    }
                    return Ok(lines.join("\n"));
                }
            }
        }
        Ok("No reminder tool available.".to_string())
    }

    pub async fn clear_reminders(&self) -> Result<String> {
        if let Some(ref rem_bin) = self.reminder_bin {
            let _ = Command::new(rem_bin)
                .arg("clear")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
            Ok("All active reminders have been cleared, sir.".to_string())
        } else {
            Ok("No reminder tool available.".to_string())
        }
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct ScheduleEventTool {
    calendar: Arc<CalendarManager>,
}

impl ScheduleEventTool {
    pub fn new(calendar: Arc<CalendarManager>) -> Self {
        Self { calendar }
    }
}

#[async_trait]
impl Tool for ScheduleEventTool {
    fn name(&self) -> &'static str {
        "schedule_event"
    }

    fn description(&self) -> &'static str {
        "Schedule an event in Google Calendar or Thunderbird (e.g. title='Team Sync', start_time='tomorrow 3pm')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "title": {
                    "type": "STRING",
                    "description": "Event summary or title."
                },
                "start_time": {
                    "type": "STRING",
                    "description": "Starting date and time (e.g., 'tomorrow 3pm', 'Friday at 10am', '2026-09-20 14:00')."
                },
                "end_time": {
                    "type": "STRING",
                    "description": "Optional end date and time."
                },
                "description": {
                    "type": "STRING",
                    "description": "Optional event details or notes."
                },
                "provider": {
                    "type": "STRING",
                    "description": "Calendar provider: 'auto', 'google', or 'thunderbird'."
                }
            },
            "required": ["title", "start_time"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let title = args["title"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("schedule_event".into(), "title string is required".into()))?;
        let start_time = args["start_time"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("schedule_event".into(), "start_time string is required".into()))?;
        let end_time = args["end_time"].as_str();
        let description = args["description"].as_str().unwrap_or("");
        let provider = args["provider"].as_str().unwrap_or("auto");

        self.calendar
            .schedule_event(title, start_time, end_time, description, provider)
            .await
    }
}

pub struct SetReminderTool {
    calendar: Arc<CalendarManager>,
}

impl SetReminderTool {
    pub fn new(calendar: Arc<CalendarManager>) -> Self {
        Self { calendar }
    }
}

#[async_trait]
impl Tool for SetReminderTool {
    fn name(&self) -> &'static str {
        "set_reminder"
    }

    fn description(&self) -> &'static str {
        "Set a desktop timer reminder with system notification and alert (e.g. 15 minutes, 'Check the oven')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "minutes": {
                    "type": "INTEGER",
                    "description": "Number of minutes until the reminder rings."
                },
                "message": {
                    "type": "STRING",
                    "description": "Reminder text or task description."
                }
            },
            "required": ["minutes", "message"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let minutes = args["minutes"]
            .as_i64()
            .ok_or_else(|| JarvisError::ToolParameter("set_reminder".into(), "minutes integer is required".into()))?;
        let message = args["message"]
            .as_str()
            .ok_or_else(|| JarvisError::ToolParameter("set_reminder".into(), "message string is required".into()))?;

        self.calendar.set_reminder(minutes, message).await
    }
}

pub struct ListRemindersTool {
    calendar: Arc<CalendarManager>,
}

impl ListRemindersTool {
    pub fn new(calendar: Arc<CalendarManager>) -> Self {
        Self { calendar }
    }
}

#[async_trait]
impl Tool for ListRemindersTool {
    fn name(&self) -> &'static str {
        "list_reminders"
    }

    fn description(&self) -> &'static str {
        "List all active upcoming desktop reminders and remaining times."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.calendar.list_reminders().await
    }
}

pub struct ClearRemindersTool {
    calendar: Arc<CalendarManager>,
}

impl ClearRemindersTool {
    pub fn new(calendar: Arc<CalendarManager>) -> Self {
        Self { calendar }
    }
}

#[async_trait]
impl Tool for ClearRemindersTool {
    fn name(&self) -> &'static str {
        "clear_reminders"
    }

    fn description(&self) -> &'static str {
        "Clear all active desktop reminders."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        self.calendar.clear_reminders().await
    }
}
