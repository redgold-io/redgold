mod example;

use extism_pdk::*;
use serde::Serialize;
use redgold_schema::{ProtoSerde, RgResult};
use redgold_schema::structs::{ExecutionInput, ExecutionResult};
use redgold_schema::transaction::amount_data;
use redgold_schema::util::lang_util::SameResult;

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


pub fn proto_example_inner(input: Vec<u8>) -> RgResult<ExecutionResult> {
    let input = ExecutionInput::proto_deserialize(input)?;
    let mut res = ExecutionResult::default();
    res.valid = true;
    res.data = amount_data(1);
    Ok(res)
}

#[plugin_fn]
pub fn proto_example(input: Vec<u8>) -> FnResult<Vec<u8>> {
    let res = proto_example_inner(input)
        .map_err(|e| ExecutionResult::from_error(e))
        .combine();
    Ok(res.proto_serialize())
}

#[test]
pub fn debug() {
    ()
}