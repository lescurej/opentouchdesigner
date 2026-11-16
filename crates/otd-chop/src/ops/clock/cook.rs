use chrono::{Local, NaiveDate};

use crate::data::{Channel, ChopData, slice_len, slice_times};
use crate::ops::ChopCtx;

use super::calendar::{
    calendar_at, fraction_of_day, fraction_of_month, fraction_of_week, fraction_of_year,
    hour_display, now_unix, parse_target_unix, Calendar, countdown_calendar, latitude_deg,
    longitude_deg,
};
use super::solar::{
    moon_phase_fraction, solar_declination_deg, sun_phase_fraction, sun_rise_set_fractions,
};

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
