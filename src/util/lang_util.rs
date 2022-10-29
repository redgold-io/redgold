use crate::schema::structs::ErrorInfo;

pub trait SameResult<T> {
    fn combine(self) -> T;
}

impl<T> SameResult<T> for Result<T, T> {
    fn combine(self) -> T {
        self.unwrap_or_else(|e| e)
    }
}

pub fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

pub trait PersistentRun {
    fn run(&mut self) -> Result<(), ErrorInfo>;
}
