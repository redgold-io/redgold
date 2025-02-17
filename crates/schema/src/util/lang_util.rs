use crate::helpers::easy_json::json_or;
use crate::structs::ErrorInfo;
use itertools::Itertools;
use serde::Serialize;

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

pub trait WithMaxLengthString {
    fn with_max_length(&self, max: usize) -> String;
    fn last_n(&self, n: usize) -> String;
}

impl WithMaxLengthString for String {
    fn with_max_length(&self, max: usize) -> String {
        if self.len() > max {
            format!("{}", &self[..max])
        } else {
            self.clone()
        }
    }

    fn last_n(&self, n: usize) -> String {
        if self.len() > n {
            format!("{}", &self[self.len() - n..])
        } else {
            self.clone()
        }
    }
}

pub fn make_ascii_titlecase(s: &mut str) -> String {
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
    return s.to_string();
}