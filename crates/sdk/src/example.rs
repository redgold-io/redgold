use extism_pdk::{plugin_fn, FnResult};

use redgold_schema::structs::{ExecutionInput, ExecutionResult, StandardData, TestContractInternalState, TestContractRequest};
use redgold_schema::{bytes_data, RgResult};

use crate::entry::with_entry_decoder;

pub fn example_request_response(
    request: TestContractRequest,
    existing_state: TestContractInternalState
) -> TestContractInternalState {
    let mut updated_state = existing_state.clone();
    if let Some(update) = request.test_contract_update {
        let mut updated = false;
        for state in updated_state.test_map.iter_mut() {
            if state.key == update.key {
                state.value = update.value.clone();
                updated = true;
            }
        }
        if !updated {
            updated_state.test_map.push(update.clone());
        }
    }
    if let Some(update) = request.test_contract_update2 {
        updated_state.latest_value = Some(update.value);
    }
    updated_state
}

pub fn zero_state() -> TestContractInternalState {
    let mut d = TestContractInternalState::default();
    d.latest_value = Some("zero".to_string());
    d
}

pub fn example_contract_main(input: ExecutionInput) -> RgResult<ExecutionResult> {

    let mut state = if let Some(i) = &input.state {
        TestContractInternalState::proto_deserialize(i.value.clone())?
    } else {
        zero_state()
    };

    if let Some(i) = input.input {
        let req = TestContractRequest::proto_deserialize(i.value)?;
        let updated_state = example_request_response(req, state);
        state = updated_state;
    }

    let mut res = ExecutionResult::default();
    let ser_state = state.proto_serialize();
    let mut data = StandardData::default();
    data.state = bytes_data(ser_state);
    res.data = Some(data);
    res.valid = true;
    Ok(res)
}


use extism_pdk::*;
use redgold_schema::proto_serde::ProtoSerde;

#[plugin_fn]
pub fn extism_entrypoint(input: Vec<u8>) -> FnResult<Vec<u8>> {
    with_entry_decoder(input, example_contract_main)
}
