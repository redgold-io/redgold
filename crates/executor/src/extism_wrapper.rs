use extism::{Context, Plugin};

use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{ExecutionInput, ExecutionResult, TestContractInternalState, TestContractRequest, TestContractUpdate2};
use redgold_schema::{bytes_data, error_info, ErrorInfoContext, RgResult};

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

pub async fn invoke_extism_wasm(
    wasm_bytes: &[u8],
    args: ExecutionInput
) -> RgResult<ExecutionResult> {
    invoke_wasm(wasm_bytes, "extism_entrypoint", args).await
}

// TODO: impl AsRef<u8>
pub async fn invoke_extism_wasm_direct(
    wasm_bytes: impl AsRef<[u8]>,
    input: &Vec<u8>,
    state: &Vec<u8>
) -> RgResult<ExecutionResult> {
    let mut args = ExecutionInput::default();
    args.input = bytes_data(input.clone());
    args.state = bytes_data(state.clone());
    invoke_wasm(wasm_bytes.as_ref(), "extism_entrypoint", args).await
}


#[ignore]
#[tokio::test]
async fn extism_direct_test() {
    // println!()
    let wasm = std::fs::read("../sdk/test_contract_guest.wasm").expect("");

    let res_g = invoke_wasm(
        &*wasm, "extism_entrypoint", ExecutionInput::default()
    ).await.unwrap();
    let gen_state = res_g.data.expect("d").state;
    let gen_state_deser = TestContractInternalState::proto_deserialize
        (gen_state.clone().expect("s").value).expect("");
    let res = gen_state_deser.json_or();
    println!("initial result genesis: {}", res);

    let mut input = ExecutionInput::default();
    let mut req = TestContractRequest::default();
    let mut update2 = TestContractUpdate2::default();
    update2.value = "UPDATED".to_string();
    req.test_contract_update2 = Some(update2);
    input.input = bytes_data(req.proto_serialize());
    input.state = gen_state.clone();

    let res = invoke_wasm(&*wasm, "extism_entrypoint", input).await.unwrap();
    println!("Exec result: {}", res.json_or());

    let done_state = res.data.clone().expect("d").state;
    let done_state_deser = TestContractInternalState::proto_deserialize
        (done_state.clone().expect("s").value).expect("");
    let resr = done_state_deser.json_or();
    println!("final result after: {}", resr);

    // let res = invoke_wasm(&*wasm, "entrypoint", input).await.unwrap();
    assert!(res.valid);
}


#[ignore]
#[tokio::test]
async fn proto_test() {
    // println!()
    let wasm = std::fs::read("../sdk/test_contract_guest.wasm").expect("");
    let input = ExecutionInput::default();
    let res = invoke_wasm(&*wasm, "extism_entrypoint", input).await.unwrap();
    println!("Exec result: {}", res.json_or());
    // let res = invoke_wasm(&*wasm, "entrypoint", input).await.unwrap();
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