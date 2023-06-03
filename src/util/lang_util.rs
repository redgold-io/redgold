use serde::Serialize;
use redgold_schema::json_or;
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


pub trait JsonCombineResult {
    fn json_or_combine(self) -> String;
}

impl<T, E> JsonCombineResult for Result<T, E>
where T: Serialize, E: Serialize {
    fn json_or_combine(self) -> String {
        self.map(|x| json_or(&x))
            .map_err(|x| json_or(&x))
            .combine()
    }
}
