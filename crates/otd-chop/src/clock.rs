use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Timelike, TimeZone};
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData, slice_len, slice_times};
use crate::ops::ChopCtx;

static PROGRAM_START_UNIX: LazyLock<f64> = LazyLock::new(|| {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
});

const SECS_PER_DAY: f64 = 86_400.0;
const SECS_PER_HOUR: f64 = 3_600.0;
const SECS_PER_MIN: f64 = 60.0;
const DAYS_PER_YEAR: f64 = 365.25;

pub fn params_clock() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "rate".into(),
        Param::float(60.0)
            .with_label("Sample Rate")
            .with_range(1.0, 48_000.0),
    );
    m.insert(
        "output".into(),
        Param::menu("fractions", &["units", "fractions", "countdown"])
            .with_label("Output")
            .with_page("Clock"),
    );
    m.insert(
        "hourformat".into(),
        Param::menu("24", &["12", "24"])
            .with_label("Hour Format")
            .with_page("Clock"),
    );
    m.insert(
        "houradjust".into(),
        Param::float(0.0)
            .with_label("Hour Adjust")
            .with_page("Clock"),
    );
    m.insert(
        "startref".into(),
        Param::menu("jan1", &["jan1", "program"])
            .with_label("Start Reference")
            .with_page("Clock"),
    );
    for (key, label) in [
        ("msec", "Millisecond"),
        ("sec", "Second"),
        ("min", "Minute"),
        ("hour", "Hour"),
        ("ampm", "AM/PM"),
        ("wday", "Day of Week"),
        ("day", "Day of Month"),
        ("yday", "Day of Year"),
        ("week", "Week"),
        ("month", "Month"),
        ("year", "Year"),
    ] {
        m.insert(
            key.into(),
            Param::bool(true).with_label(label).with_page("Clock"),
        );
    }
    m.insert(
        "latitude1".into(),
        Param::float(43.0)
            .with_label("Latitude (deg)")
            .with_page("Solar"),
    );
    m.insert(
        "latitude2".into(),
        Param::float(39.0)
            .with_label("Latitude (min)")
            .with_page("Solar"),
    );
    m.insert(
        "northsouth".into(),
        Param::menu("north", &["north", "south"])
            .with_label("N/S")
            .with_page("Solar"),
    );
    m.insert(
        "longitude1".into(),
        Param::float(79.0)
            .with_label("Longitude (deg)")
            .with_page("Solar"),
    );
    m.insert(
        "longitude2".into(),
        Param::float(23.0)
            .with_label("Longitude (min)")
            .with_page("Solar"),
    );
    m.insert(
        "eastwest".into(),
        Param::menu("west", &["east", "west"])
            .with_label("E/W")
            .with_page("Solar"),
    );
    for (key, label) in [
        ("moonphase", "Moon Phase"),
        ("sunphase", "Sun Phase"),
        ("sunrise", "Sunrise"),
        ("sunset", "Sunset"),
        ("declination", "Declination"),
    ] {
        m.insert(
            key.into(),
            Param::bool(false).with_label(label).with_page("Solar"),
        );
    }
    m
}

#[derive(Clone, Copy, Debug)]
struct Calendar {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
    msec: u32,
    wday: u32,
    yday: u32,
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
            msec: (dt.nanosecond() / 1_000_000) as u32,
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

fn latitude_deg(c: &ChopCtx) -> f64 {
    let mut lat = c.f("latitude1") as f64 + c.f("latitude2") as f64 / 60.0;
    if c.menu("northsouth") == 1 {
        lat = -lat;
    }
    lat
}

fn longitude_deg(c: &ChopCtx) -> f64 {
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

fn fraction_of_day(h: u32, m: u32, s: u32, ms: u32) -> f64 {
    (h as f64 * SECS_PER_HOUR
        + m as f64 * SECS_PER_MIN
        + s as f64
        + ms as f64 / 1000.0)
        / SECS_PER_DAY
}

fn fraction_of_year(cal: &Calendar) -> f64 {
    let diy = days_in_year(cal.year);
    (cal.yday as f64 + fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec)) / diy
}

fn fraction_of_month(cal: &Calendar) -> f64 {
    let dim = days_in_month(cal.year, cal.month);
    ((cal.day - 1) as f64 + fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec)) / dim
}

fn fraction_of_week(cal: &Calendar) -> f64 {
    (cal.wday as f64 + fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec)) / 7.0
}

fn hour_display(cal: &Calendar, hour_12: bool) -> u32 {
    if hour_12 {
        cal.hour % 12
    } else {
        cal.hour
    }
}

fn read_optional_channel(data: &ChopData, name: &str) -> Option<f32> {
    data.value(name)
}

fn parse_target_unix(data: &ChopData) -> Option<f64> {
    let year = read_optional_channel(data, "year").map(|y| y.floor() as i32);
    let year = year.map(|y| if y < 100 { 2000 + y } else { y })?;
    let month = read_optional_channel(data, "month")
        .map(|m| m.max(1.0).min(12.0) as u32)
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

fn now_unix(hour_adjust: f64) -> f64 {
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

fn calendar_at(
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
        cal.month = m.max(1.0).min(12.0) as u32;
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

fn solar_declination_deg(day_of_year: u32) -> f64 {
    let g = 2.0 * std::f64::consts::PI * (day_of_year as f64 - 1.0) / 365.0;
    23.45 * g.sin()
}

fn sun_rise_set_fractions(lat_deg: f64, lon_deg: f64, date: NaiveDate) -> (f64, f64) {
    let zenith = 90.833_f64.to_radians();
    let lat = lat_deg.to_radians();
    let day = date.ordinal0() as f64 + 1.0;
    let decl = solar_declination_deg(day as u32).to_radians();
    let cos_h =
        (zenith.cos() - lat.sin() * decl.sin()) / (lat.cos() * decl.cos()).clamp(1e-6, 1.0);
    if cos_h.abs() > 1.0 {
        return (0.0, 0.0);
    }
    let h = cos_h.acos();
    let lw = lon_deg / 15.0;
    let rise = 12.0 - h.to_degrees() / 15.0 - lw;
    let set = 12.0 + h.to_degrees() / 15.0 - lw;
    let norm = |hours: f64| (hours / 24.0).rem_euclid(1.0);
    (norm(rise), norm(set))
}

fn moon_phase_fraction(unix: f64) -> f32 {
    let days = unix / SECS_PER_DAY;
    let new_moon = 6.0;
    let synodic = 29.530588853;
    ((days - new_moon) / synodic).rem_euclid(1.0) as f32
}

fn sun_phase_fraction(now_frac: f64, rise: f64, set: f64) -> f32 {
    if set <= rise {
        return 0.0;
    }
    if now_frac >= rise && now_frac <= set {
        ((now_frac - rise) / (set - rise)) as f32
    } else if now_frac > set {
        let span = (1.0 - set) + rise;
        (1.0 - (now_frac - set) / span).clamp(0.0, 1.0) as f32
    } else {
        (now_frac / rise).clamp(0.0, 1.0) as f32
    }
}

fn countdown_calendar(remaining: f64) -> Calendar {
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

struct ChannelValues {
    msec: f32,
    sec: f32,
    min: f32,
    hour: f32,
    ampm: f32,
    wday: f32,
    day: f32,
    yday: f32,
    week: f32,
    month: f32,
    year: f32,
    moonphase: f32,
    sunphase: f32,
    sunrise: f32,
    sunset: f32,
    declination: f32,
}

fn eval_channels(
    c: &ChopCtx,
    unix: f64,
    cal: &Calendar,
    output: usize,
    hour_12: bool,
    startref: usize,
) -> ChannelValues {
    let date = NaiveDate::from_ymd_opt(cal.year, cal.month.max(1), cal.day.max(1))
        .unwrap_or_else(|| Local::now().date_naive());
    let lat = latitude_deg(c);
    let lon = longitude_deg(c);
    let (rise, set) = sun_rise_set_fractions(lat, lon, date);
    let now_frac = fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec);

    let (msec, sec, min, hour, ampm, wday, day, yday, week, month, year) = if output == 2 {
        let rem = unix;
        let cd = countdown_calendar(rem);
        (
            cd.msec as f32,
            cd.sec as f32,
            cd.min as f32,
            cd.hour as f32,
            0.0,
            0.0,
            cd.day as f32,
            cd.yday as f32,
            (cd.yday / 7) as f32,
            0.0,
            0.0,
        )
    } else if output == 1 {
        let sec_f = cal.sec as f64 + cal.msec as f64 / 1000.0;
        let min_f = cal.min as f64 + sec_f / 60.0;
        let hour_f = cal.hour as f64 + min_f / 60.0;
        (
            (cal.msec as f64 / 1000.0) as f32,
            (sec_f / 60.0) as f32,
            (min_f / 60.0) as f32,
            (hour_f / 24.0) as f32,
            if cal.hour < 12 { 0.0 } else { 1.0 },
            fraction_of_day(cal.hour, cal.min, cal.sec, cal.msec) as f32,
            0.0,
            0.0,
            fraction_of_week(cal) as f32,
            fraction_of_month(cal) as f32,
            fraction_of_year(cal) as f32,
        )
    } else {
        let hour_u = hour_display(cal, hour_12);
        let year_u = if startref == 1 {
            cal.year
        } else {
            cal.year - 2000
        };
        (
            cal.msec as f32,
            cal.sec as f32,
            cal.min as f32,
            hour_u as f32,
            if cal.hour < 12 { 0.0 } else { 1.0 },
            cal.wday as f32,
            cal.day as f32,
            cal.yday as f32,
            (cal.yday / 7).min(51) as f32,
            cal.month as f32,
            year_u as f32,
        )
    };

    ChannelValues {
        msec,
        sec,
        min,
        hour,
        ampm,
        wday,
        day,
        yday,
        week,
        month,
        year,
        moonphase: moon_phase_fraction(unix),
        sunphase: sun_phase_fraction(now_frac, rise, set),
        sunrise: rise as f32,
        sunset: set as f32,
        declination: solar_declination_deg(cal.yday + 1) as f32,
    }
}

pub fn cook_clock(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let output = c.menu("output");
    let hour_12 = c.menu("hourformat") == 0;
    let startref = c.menu("startref");
    let hour_adjust = c.f("houradjust") as f64;

    let target_input = c.input(0);
    let reference_input = c.input(1);

    let mut channels: Vec<Channel> = Vec::new();

    for t in slice_times(c.time, n) {
        let _ = t;
        let now = if output == 2 {
            let target = parse_target_unix(target_input).unwrap_or(now_unix(hour_adjust));
            let reference = parse_target_unix(reference_input)
                .or_else(|| Some(now_unix(hour_adjust)))
                .unwrap_or(now_unix(hour_adjust));
            target - reference
        } else {
            now_unix(hour_adjust)
        };

        let cal = calendar_at(now, startref, target_input);
        let v = eval_channels(c, now, &cal, output, hour_12, startref);

        let push = |channels: &mut Vec<Channel>, name: &str, val: f32| {
            if let Some(ch) = channels.iter_mut().find(|ch| ch.name == name) {
                ch.samples.push(val);
            } else {
                channels.push(Channel::new(name, vec![val]));
            }
        };

        if c.b("msec") {
            push(&mut channels, "msec", v.msec);
        }
        if c.b("sec") {
            push(&mut channels, "sec", v.sec);
        }
        if c.b("min") {
            push(&mut channels, "min", v.min);
        }
        if c.b("hour") {
            push(&mut channels, "hour", v.hour);
        }
        if c.b("ampm") {
            push(&mut channels, "ampm", v.ampm);
        }
        if c.b("wday") {
            push(&mut channels, "wday", v.wday);
        }
        if c.b("day") && output != 1 {
            push(&mut channels, "day", v.day);
        }
        if c.b("yday") && output != 1 {
            push(&mut channels, "yday", v.yday);
        }
        if c.b("week") {
            push(&mut channels, "week", v.week);
        }
        if c.b("month") {
            push(&mut channels, "month", v.month);
        }
        if c.b("year") {
            push(&mut channels, "year", v.year);
        }
        if c.b("moonphase") {
            push(&mut channels, "moonphase", v.moonphase);
        }
        if c.b("sunphase") {
            push(&mut channels, "sunphase", v.sunphase);
        }
        if c.b("sunrise") {
            push(&mut channels, "sunrise", v.sunrise);
        }
        if c.b("sunset") {
            push(&mut channels, "sunset", v.sunset);
        }
        if c.b("declination") {
            push(&mut channels, "declination", v.declination);
        }
    }

    for ch in &mut channels {
        if ch.samples.len() < n {
            let last = ch.last();
            ch.samples.resize(n, last);
        }
    }

    ChopData::new(channels, rate, true)
}
