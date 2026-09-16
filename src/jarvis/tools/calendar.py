"""Calendar scheduling and reminder integration for Jarvis."""

import asyncio
import datetime
import json
from pathlib import Path
import re
import shutil
import urllib.parse
from typing import Optional
from jarvis.core.logger import log

EVENTS_DIR = Path.home() / ".local" / "share" / "jarvis" / "events"


def parse_event_datetime(time_str: str) -> datetime.datetime:
    """Parse relative or absolute date-time strings into a datetime object."""
    now = datetime.datetime.now()
    clean = time_str.strip().lower()

    for fmt in ("%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d"):
        try:
            dt = datetime.datetime.strptime(clean, fmt)
            if fmt == "%Y-%m-%d":
                return dt.replace(hour=9, minute=0)
            return dt
        except ValueError:
            pass

    day_offset = 0
    if "day after tomorrow" in clean:
        day_offset = 2
        clean = clean.replace("day after tomorrow", "").strip()
    elif "tomorrow" in clean:
        day_offset = 1
        clean = clean.replace("tomorrow", "").strip()
    elif "today" in clean:
        day_offset = 0
        clean = clean.replace("today", "").strip()
    else:
        weekdays = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
        for idx, day in enumerate(weekdays):
            if day in clean:
                days_ahead = (idx - now.weekday()) % 7
                if days_ahead == 0:
                    days_ahead = 7
                day_offset = days_ahead
                clean = clean.replace(day, "").replace("next", "").strip()
                break

    target_date = (now + datetime.timedelta(days=day_offset)).date()
    clean = clean.replace("at", "").strip()

    match_12h = re.search(r"(\d{1,2})(?::(\d{2}))?\s*(am|pm)", clean)
    if match_12h:
        hour = int(match_12h.group(1))
        minute = int(match_12h.group(2) or 0)
        meridiem = match_12h.group(3)
        if meridiem == "pm" and hour != 12:
            hour += 12
        elif meridiem == "am" and hour == 12:
            hour = 0
        return datetime.datetime.combine(target_date, datetime.time(hour, minute))

    match_24h = re.search(r"(\d{1,2}):(\d{2})", clean)
    if match_24h:
        hour = int(match_24h.group(1))
        minute = int(match_24h.group(2))
        return datetime.datetime.combine(target_date, datetime.time(hour, minute))

    match_num = re.search(r"\b(\d{1,2})\b", clean)
    if match_num:
        hour = int(match_num.group(1))
        if hour < 8:
            hour += 12
        return datetime.datetime.combine(target_date, datetime.time(hour, 0))

    return (now + datetime.timedelta(days=1)).replace(hour=9, minute=0, second=0, microsecond=0)


class CalendarManager:
    """Handles calendar event scheduling and systemd-backed desktop reminders."""

    def __init__(self):
        self._browser_bin = shutil.which("omarchy-launch-browser") or shutil.which("xdg-open") or "xdg-open"
        self._reminder_bin = shutil.which("omarchy-reminder")
        self._thunderbird_bin = shutil.which("thunderbird")
        EVENTS_DIR.mkdir(parents=True, exist_ok=True)

    async def schedule_event(
        self,
        title: str,
        start_time: str,
        end_time: Optional[str] = None,
        description: str = "",
        provider: str = "auto",
    ) -> str:
        """Schedule a calendar event via Google Calendar or Thunderbird / iCalendar."""
        dt_start = parse_event_datetime(start_time)
        if end_time:
            dt_end = parse_event_datetime(end_time)
        else:
            dt_end = dt_start + datetime.timedelta(hours=1)

        start_str = dt_start.strftime("%Y%m%dT%H%M%S")
        end_str = dt_end.strftime("%Y%m%dT%H%M%S")
        nice_start = dt_start.strftime("%A, %B %d at %I:%M %p")

        # Generate Google Calendar Template URL
        encoded_title = urllib.parse.quote(title)
        encoded_desc = urllib.parse.quote(description)
        gcal_url = (
            f"https://calendar.google.com/calendar/render?action=TEMPLATE"
            f"&text={encoded_title}&details={encoded_desc}&dates={start_str}/{end_str}"
        )

        # Generate RFC 5545 iCalendar .ics file
        ics_filename = f"{dt_start.strftime('%Y%m%d_%H%M')}_{re.sub(r'[^a-zA-Z0-9]', '_', title)[:20]}.ics"
        ics_path = EVENTS_DIR / ics_filename
        ics_content = (
            "BEGIN:VCALENDAR\n"
            "VERSION:2.0\n"
            "PRODID:-//Jarvis Assistant//EN\n"
            "BEGIN:VEVENT\n"
            f"UID:{dt_start.timestamp()}@jarvis\n"
            f"DTSTAMP:{datetime.datetime.now().strftime('%Y%m%dT%H%M%S')}\n"
            f"DTSTART:{start_str}\n"
            f"DTEND:{end_str}\n"
            f"SUMMARY:{title}\n"
            f"DESCRIPTION:{description}\n"
            "STATUS:CONFIRMED\n"
            "END:VEVENT\n"
            "END:VCALENDAR\n"
        )
        ics_path.write_text(ics_content)

        log.info(f"[cyan]Scheduling event '{title}' for {nice_start}[/cyan]")

        if provider.lower() in ("thunderbird", "local") and self._thunderbird_bin:
            await asyncio.create_subprocess_exec(
                self._thunderbird_bin,
                "-file",
                str(ics_path),
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            return f"Scheduled '{title}' for {nice_start} and opened in Thunderbird."
        else:
            # Open Google Calendar with all fields prefilled
            await asyncio.create_subprocess_exec(
                self._browser_bin,
                gcal_url,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            return f"Scheduled '{title}' for {nice_start}. Opened Google Calendar for confirmation, sir."

    async def set_reminder(self, minutes: int, message: str) -> str:
        """Set a timer reminder using Omarchy systemd reminders."""
        if minutes <= 0:
            minutes = 5

        if self._reminder_bin:
            proc = await asyncio.create_subprocess_exec(
                self._reminder_bin,
                str(minutes),
                message,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            await proc.communicate()
            remind_time = (datetime.datetime.now() + datetime.timedelta(minutes=minutes)).strftime("%I:%M %p")
            return f"Reminder set for {minutes} minutes from now (at {remind_time}): '{message}', sir."
        else:
            return f"Reminder system not available on this system."

    async def list_reminders(self) -> str:
        """List active pending reminders."""
        if not self._reminder_bin:
            return "No reminder tool available."

        proc = await asyncio.create_subprocess_exec(
            self._reminder_bin,
            "show",
            "--json",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, _ = await proc.communicate()
        try:
            data = json.loads(stdout.decode())
            reminders = data.get("reminders", [])
            if not reminders:
                return "You have no outstanding reminders, sir."
            lines = ["Upcoming reminders:"]
            for r in reminders:
                label = r.get("label", "Reminder")
                remaining = r.get("remaining", "")
                at_time = r.get("atTime", "")
                lines.append(f"- '{label}' at {at_time} (in {remaining})")
            return "\n".join(lines)
        except Exception:
            return "No active reminders found."

    async def clear_reminders(self) -> str:
        """Clear all active reminders."""
        if not self._reminder_bin:
            return "No reminder tool available."

        proc = await asyncio.create_subprocess_exec(
            self._reminder_bin,
            "clear",
            stdout=asyncio.subprocess.DEVNULL,
            stderr=asyncio.subprocess.DEVNULL,
        )
        await proc.wait()
        return "All active reminders have been cleared, sir."
