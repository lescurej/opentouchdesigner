use otd_core::indexmap::IndexMap;
use otd_core::Param;

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
