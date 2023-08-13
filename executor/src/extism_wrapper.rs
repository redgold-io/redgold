use std::ops::Add;
use extism::{Context, Plugin};
use redgold_schema::{error_info, ErrorInfoContext, ProtoSerde, RgResult, structs};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{ExecutionInput, ExecutionResult};

pub async fn invoke_wasm(
    wasm_bytes: &[u8],
    function_name: impl Into<String>,
    args: ExecutionInput
) -> RgResult<ExecutionResult> {
    let context = Context::new();
    let mut plugin = Plugin::new(
        &context,
        wasm_bytes,
        vec![],
        false
    ).map_err(|e|
        error_info(
            format!("Unable to build plugin while invoking wasm {}", e.to_string())))?;

    let fname = function_name.into();
    let has = plugin.has_function(fname.clone());
    if !has {
        return Err(error_info(format!("Function not found {}", fname.clone())))?
    }
    let data = plugin.call(fname.clone(), args.proto_serialize())
        .map_err(|e|
            error_info(
                format!("Error calling function {}", e.to_string())))
        .add(fname.clone())?;
    ExecutionResult::proto_deserialize(data.to_vec())
}

#[ignore]
#[tokio::test]
async fn proto_test() {
    let wasm = std::fs::read("../../sdk/test_contract_guest.wasm").expect("");
    let input = ExecutionInput::default();
    let res = invoke_wasm(&*wasm, "proto_example", input).await.unwrap();
    assert!(res.valid);
}

#[ignore]
#[test]
fn debug_test() {

    let context = Context::new();
    // let wasm = include_bytes!("code.wasm");
    let wasm = std::fs::read("../../sdk/test_contract_guest.wasm").expect("");
    // let wasm = include_bytes!("../../sdk/extism_test.wasm");

    // NOTE: if you encounter an error such as:
    // "Unable to load plugin: unknown import: wasi_snapshot_preview1::fd_write has not been defined"
    // change `false` to `true` in the following function to provide WASI imports to your plugin.
    let mut plugin = Plugin::new(&context, wasm, vec![], false).unwrap();
    let has = plugin.has_function("count_vowels");
    println!("has: {:?}", has);
    let data = plugin.call("count_vowels", "this is a test");
    println!("data: {:?}", data);
    // assert_eq!(data, b"{\"count\": 4}");
}