impl Param {
    pub fn float(default: f64) -> Self {
        Param::new(default)
    }
    pub fn int(default: i64) -> Self {
        Param::new(default)
    }
    pub fn bool(default: bool) -> Self {
        Param::new(default)
    }
    pub fn str(default: &str) -> Self {
        Param::new(default)
    }
    pub fn rgba(default: [f64; 4]) -> Self {
        Param::new(default)
    }
    pub fn xyz(default: [f64; 3]) -> Self {
        Param::new(default)
    }
    pub fn menu(default: &str, items: &[&str]) -> Self {
        Param::new(default).with_menu(items)
    }
    pub fn select(default: &str, items: &[(&str, &str)]) -> Self {
        let mut p = Param::new(Value::Select(default.to_string()));
        p.menu = Some(items.iter().map(|(o, v)| SelectOption::new(o, v)).collect());
        p
    }
}
