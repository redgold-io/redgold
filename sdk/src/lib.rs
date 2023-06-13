mod example;

use extism_pdk::*;
use serde::Serialize;

const VOWELS: &[char] = &['a', 'A', 'e', 'E', 'i', 'I', 'o', 'O', 'u', 'U'];

#[derive(Serialize)]
struct TestOutput {
    pub count: i32,
    pub config: String,
    pub a: String,
}

#[plugin_fn]
pub fn count_vowels(input: String) -> FnResult<String> {
    // let mut count = 0;
    // for ch in input.chars() {
    //     if VOWELS.contains(&ch) {
    //         count += 1;
    //     }
    // }
    // Theres a bug causing a panic somewhere in the normal code here
    // set_var!("a", "this is var a")?;
    //
    // let a = var::get("a")?.expect("variable 'a' set");
    // let a = String::from_utf8(a).expect("string from varible value");
    // let config = config::get("thing").expect("'thing' key set in config");
    let result = format!("{} plus {}", input, "asdf");
    // let output = TestOutput { count, config, a };
    // Ok(Json(output))
    Ok(result)
}