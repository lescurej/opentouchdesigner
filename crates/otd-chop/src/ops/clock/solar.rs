use chrono::{Datelike, NaiveDate};

use super::common::*;

pub(super) fn solar_declination_deg(day_of_year: u32) -> f64 {
    let g = 2.0 * std::f64::consts::PI * (day_of_year as f64 - 1.0) / 365.0;
    23.45 * g.sin()
}

pub(super) fn sun_rise_set_fractions(lat_deg: f64, lon_deg: f64, date: NaiveDate) -> (f64, f64) {
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

pub(super) fn moon_phase_fraction(unix: f64) -> f32 {
    let days = unix / SECS_PER_DAY;
    let new_moon = 6.0;
    let synodic = 29.530588853;
    ((days - new_moon) / synodic).rem_euclid(1.0) as f32
}

pub(super) fn sun_phase_fraction(now_frac: f64, rise: f64, set: f64) -> f32 {
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

