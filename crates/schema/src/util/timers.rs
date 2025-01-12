use log::info;

pub struct PerfTimer {
    start: std::time::Instant,
    latest: std::time::Instant,
    map: std::collections::HashMap<String, i64>,
    name: Option<String>
}

impl PerfTimer {
    pub fn new() -> Self {
        let start = std::time::Instant::now();
        Self {
            start,
            latest: start,
            map: Default::default(),
            name: None,
        }
    }

    pub fn named(name: impl Into<String>) -> Self {
        let mut timer = Self::new();
        timer.name = Some(name.into());
        timer
    }
    pub fn mark(&mut self) {
        self.latest = std::time::Instant::now();
    }

    pub fn record(&mut self, name: impl Into<String>) {
        let millis = self.millis();
        self.map.insert(name.into(), millis);
    }

    pub fn millis(&mut self) -> i64 {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.latest);
        self.latest = now;
        let millis = elapsed.as_millis() as i64;
        millis
    }

    pub fn seconds(&self) -> f64 {
        let elapsed = self.start.elapsed();
        elapsed.as_secs_f64()
    }

    pub fn log(&self) {
        info!("Timer {} took {} seconds", self.name.clone().unwrap_or("unnamed".to_string()), self.seconds());
    }

}
