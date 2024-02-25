use itertools::Itertools;
use serde::Serialize;
use crate::json_or;
use crate::structs::ErrorInfo;

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

pub trait VecAddBy<T, K> {
    fn add_by(&mut self, other: T, func: fn(&T) -> &K) -> Vec<T>;
}

impl<T, K> VecAddBy<T, K> for Vec<T> where K : PartialEq, T: Clone {
    fn add_by(&mut self, other: T, func: fn(&T) -> &K) -> Vec<T> {
        let k = func(&other);
        let mut res = self.iter().filter(|x| func(x) != k)
            .map(|x| x.clone()
            ).collect_vec();
        res.push(other);
        res
    }
}

pub trait AnyPrinter{
    fn print(&self);
}

impl<T> AnyPrinter for T where T: std::fmt::Display {
    fn print(&self) {
        println!("{}", self);
    }
}