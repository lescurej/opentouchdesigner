use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Timelike, TimeZone};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::data::ChopData;
use crate::ops::ChopCtx;

use super::common::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Calendar {
    pub(crate) year: i32,
    pub(crate) month: u32,
    pub(crate) day: u32,
    pub(crate) hour: u32,
    pub(crate) min: u32,
    pub(crate) sec: u32,
    pub(crate) msec: u32,
    pub(crate) wday: u32,
    pub(crate) yday: u32,
}

impl Calendar {
    fn from_local(dt: NaiveDateTime) -> Self {
        let date = dt.date();
        let wday = date.weekday().num_days_from_monday();
        let yday = date.ordinal0();
        Calendar {
            year: date.year(),
            month: date.month(),
            day: date.day(),
            hour: dt.hour(),
            min: dt.minute(),
            sec: dt.second(),
            msec: dt.nanosecond() / 1_000_000,
            wday,
            yday,
        }
    }

    fn from_program_elapsed(secs: f64) -> Self {
        let mut t = secs.max(0.0);
        let year = (t / (DAYS_PER_YEAR * SECS_PER_DAY)).floor() as i32;
        t -= year as f64 * DAYS_PER_YEAR * SECS_PER_DAY;
        let yday = (t / SECS_PER_DAY).floor() as u32;
        t -= yday as f64 * SECS_PER_DAY;
        let hour = (t / SECS_PER_HOUR).floor() as u32;
        t -= hour as f64 * SECS_PER_HOUR;
        let min = (t / SECS_PER_MIN).floor() as u32;
        t -= min as f64 * SECS_PER_MIN;
        let sec = t.floor() as u32;
        t -= sec as f64;
        let msec = (t * 1000.0).round() as u32;
        let month = (yday / 30).min(11) + 1;
        let day = (yday % 30) + 1;
        let wday = (yday + 3) % 7;
        Calendar {
            year,
            month,
            day,
            hour,
            min,
            sec,
            msec,
            wday,
            yday,
        }
    }
}

pub(super) fn latitude_deg(c: &ChopCtx) -> f64 {
    let mut lat = c.f("latitude1") as f64 + c.f("latitude2") as f64 / 60.0;
    if c.menu("northsouth") == 1 {
        lat = -lat;
    }
    lat
}

pub(super) fn longitude_deg(c: &ChopCtx) -> f64 {
    let mut lon = c.f("longitude1") as f64 + c.f("longitude2") as f64 / 60.0;
    if c.menu("eastwest") == 1 {
        lon = -lon;
    }
    lon
}

fn days_in_year(year: i32) -> f64 {
    if NaiveDate::from_ymd_opt(year, 2, 29).is_some() {
        366.0
    } else {
        365.0
    }
}

fn days_in_month(year: i32, month: u32) -> f64 {
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1))
        .and_then(|next| NaiveDate::from_ymd_opt(year, month, 1).map(|cur| (next - cur).num_days()))
        .unwrap_or(30) as f64
}

pub(super) fn fraction_of_day(h: u32, m: u32, s: u32, ms: u32) -> f64 {
    (h as f64 * SECS_PER_HOUR
        + m as f64 * SECS_PER_MIN
        + s as f64
        + ms as f64 / 1000.0)
        / SECS_PER_DAY
}

pub(super) fn fraction_of_year(cal: &Calendar) -> f64 {
    let diy = days_in_year(cal.year);
    (cal.yday as f64 + fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec)) / diy
}

pub(super) fn fraction_of_month(cal: &Calendar) -> f64 {
    let dim = days_in_month(cal.year, cal.month);
    ((cal.day - 1) as f64 + fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec)) / dim
}

pub(super) fn fraction_of_week(cal: &Calendar) -> f64 {
    (cal.wday as f64 + fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec)) / 7.0
}

pub(super) fn hour_display(cal: &Calendar, hour_12: bool) -> u32 {
    if hour_12 {
        cal.hour % 12
    } else {
        cal.hour
    }
}

fn read_optional_channel(data: &ChopData, name: &str) -> Option<f32> {
    data.value(name)
}

pub(super) fn parse_target_unix(data: &ChopData) -> Option<f64> {
    let year = read_optional_channel(data, "year").map(|y| y.floor() as i32);
    let year = year.map(|y| if y < 100 { 2000 + y } else { y })?;
    let month = read_optional_channel(data, "month")
        .map(|m| m.clamp(1.0, 12.0) as u32)
        .unwrap_or(1);
    let day = read_optional_channel(data, "day")
        .map(|d| d.max(1.0) as u32)
        .unwrap_or(1);
    let hour = read_optional_channel(data, "hour")
        .map(|h| h.max(0.0) as u32)
        .unwrap_or(0);
    let min = read_optional_channel(data, "min")
        .map(|m| m.max(0.0) as u32)
        .unwrap_or(0);
    let sec = read_optional_channel(data, "sec")
        .map(|s| s.max(0.0) as u32)
        .unwrap_or(0);
    let msec = read_optional_channel(data, "msec")
        .map(|m| m.max(0.0) as u32)
        .unwrap_or(0);
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let dt = date.and_hms_milli_opt(hour, min, sec, msec)?;
    Local
        .from_local_datetime(&dt)
        .single()
        .map(|ldt| ldt.timestamp() as f64 + ldt.timestamp_subsec_millis() as f64 / 1000.0)
}

pub(super) fn now_unix(hour_adjust: f64) -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64() + hour_adjust * SECS_PER_HOUR)
        .unwrap_or(0.0)
}

fn unix_to_local(unix: f64) -> NaiveDateTime {
    let secs = unix.floor() as i64;
    let nsec = ((unix - secs as f64) * 1e9).round() as u32;
    Local
        .timestamp_opt(secs, nsec)
        .single()
        .map(|ldt| ldt.naive_local())
        .unwrap_or_else(|| Local::now().naive_local())
}

pub(super) fn calendar_at(
    unix: f64,
    startref: usize,
    overrides: &ChopData,
) -> Calendar {
    let mut cal = if startref == 1 {
        Calendar::from_program_elapsed(unix - *PROGRAM_START_UNIX)
    } else {
        Calendar::from_local(unix_to_local(unix))
    };
    if let Some(y) = read_optional_channel(overrides, "year") {
        let y = y.floor() as i32;
        cal.year = if startref == 1 {
            y
        } else if y < 100 {
            2000 + y
        } else {
            y
        };
    }
    if let Some(m) = read_optional_channel(overrides, "month") {
        cal.month = m.clamp(1.0, 12.0) as u32;
    }
    if let Some(d) = read_optional_channel(overrides, "day") {
        cal.day = d.max(1.0) as u32;
    }
    if let Some(h) = read_optional_channel(overrides, "hour") {
        cal.hour = h.max(0.0) as u32 % 24;
    }
    if let Some(m) = read_optional_channel(overrides, "min") {
        cal.min = m.max(0.0) as u32 % 60;
    }
    if let Some(s) = read_optional_channel(overrides, "sec") {
        cal.sec = s.max(0.0) as u32 % 60;
    }
    if let Some(ms) = read_optional_channel(overrides, "msec") {
        cal.msec = ms.max(0.0) as u32 % 1000;
    }
    if let Some(w) = read_optional_channel(overrides, "wday") {
        cal.wday = w.max(0.0) as u32 % 7;
    }
    if let Some(y) = read_optional_channel(overrides, "yday") {
        cal.yday = y.max(0.0) as u32;
    }
    cal
}

pub(super) fn countdown_calendar(remaining: f64) -> Calendar {
    let mut t = remaining.max(0.0);
    let sec = (t % SECS_PER_MIN).floor() as u32;
    t = (t / SECS_PER_MIN).floor();
    let min = (t % SECS_PER_HOUR).floor() as u32;
    t = (t / SECS_PER_HOUR).floor();
    let hour = (t % 24.0).floor() as u32;
    t = (t / 24.0).floor();
    let day = t as u32;
    Calendar {
        year: 0,
        month: 0,
        day,
        hour,
        min,
        sec,
        msec: ((remaining % 1.0) * 1000.0).round() as u32,
        wday: 0,
        yday: day,
    }
}
