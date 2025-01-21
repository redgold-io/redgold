use extism_pdk::FnResult;
use redgold_schema::RgResult;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{ExecutionInput, ExecutionResult};
use redgold_schema::util::lang_util::SameResult;

pub fn with_entry_decoder<F: FnOnce(ExecutionInput) -> RgResult<ExecutionResult>>(
    input: Vec<u8>, func: F
) -> FnResult<Vec<u8>> {
    let result = ExecutionInput::proto_deserialize(input).and_then(func);
    let err_handled = result
        .map_err(|e| ExecutionResult::from_error(e))
        .combine();
    Ok(err_handled.proto_serialize())
}