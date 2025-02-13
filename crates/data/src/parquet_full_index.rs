use polars::datatypes::{AnyValue, DataType, Field, TimeUnit};
use polars::frame::row::Row;
use polars::prelude::Schema;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::Transaction;
use redgold_schema::RgResult;

pub fn parquet_schema(_time: Option<i64>) -> Schema {
    Schema::from_iter(vec![
        Field::new("time", DataType::Datetime(TimeUnit::Milliseconds, None)),
        Field::new("sponsored_time", DataType::Datetime(TimeUnit::Milliseconds, None)),
        Field::new("transaction_proto", DataType::Binary),
        Field::new("hash", DataType::Binary),
        Field::new("signable_hash", DataType::Binary),
        Field::new("signed_hash", DataType::Binary),
        Field::new("counterparty_hash", DataType::Binary),
        Field::new("confirmation_hash", DataType::Binary),
        Field::new("first_input_address", DataType::Binary),
        Field::new("first_output_address", DataType::Binary),
        Field::new("transaction_type", DataType::Int32),
        Field::new("is_test", DataType::Boolean),
        Field::new("total_amount", DataType::Int64),
        Field::new("first_amount", DataType::Int64),
        Field::new("remainder_amount", DataType::Int64),
        Field::new("contract_type", DataType::Int32),
    ])
}


fn translate_tx<'a>(tx: &Transaction) -> RgResult<Row<'a>> {
    let time = tx.time()?.clone();
    let sponsored_time = tx.sponsored_time().ok();
    let transaction_proto = tx.proto_serialize();
    let hash = tx.hash_proto_bytes();
    let signable_hash = tx.signable_hash().vec();
    let signed_hash = tx.signed_hash().vec();
    // TODO: Distinguish these cases
    let counterparty_hash: Option<Vec<u8>> = None;
    let confirmation_hash: Option<Vec<u8>> = None;
    let first_input_address = tx.first_input_address().map(|a| a.proto_serialize());
    let first_output_address = tx.first_output_address_non_input_or_fee().map(|a| a.proto_serialize());
    let transaction_type = tx.transaction_type().ok().map(|t| t as i32);
    let is_test = tx.is_test();
    let total_amount = tx.total_output_amount();
    let first_amount = tx.first_output_amount_i64();
    let remainder_amount = tx.remainder_amount();
    let contract_type = tx.first_contract_type().map(|c| c as i32);

    Ok(Row::new(vec![
        AnyValue::Datetime(time, TimeUnit::Milliseconds, &None),
        AnyValue::from(sponsored_time.map(|t| AnyValue::Datetime(t, TimeUnit::Milliseconds, &None))),
        AnyValue::BinaryOwned(transaction_proto),
        AnyValue::BinaryOwned(hash.to_vec()),
        AnyValue::BinaryOwned(signable_hash.to_vec()),
        AnyValue::BinaryOwned(signed_hash.to_vec()),
        AnyValue::from(counterparty_hash.map(|h| AnyValue::BinaryOwned(h.to_vec()))),
        AnyValue::from(confirmation_hash.map(|h| AnyValue::BinaryOwned(h.to_vec()))),
        AnyValue::from(first_input_address.map(|a| AnyValue::BinaryOwned(a))),
        AnyValue::from(first_output_address.map(|a| AnyValue::BinaryOwned(a))),
        AnyValue::from(transaction_type),
        AnyValue::Boolean(is_test),
        AnyValue::Int64(total_amount),
        AnyValue::from(first_amount.map(|a| AnyValue::Int64(a))),
        AnyValue::Int64(remainder_amount),
        AnyValue::from(contract_type),
    ]))
}
