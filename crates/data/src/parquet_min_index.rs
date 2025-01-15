use polars::datatypes::{AnyValue, DataType, Field, TimeUnit};
use polars::frame::row::Row;
use polars::prelude::Schema;
use redgold_schema::RgResult;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::Transaction;

pub fn transaction_simple_parquet_schema(time: Option<i64>) -> Schema {
    Schema::from_iter(vec![
        Field::new("time", DataType::Datetime(TimeUnit::Milliseconds, None)),
        Field::new("hash", DataType::Binary),
        Field::new("transaction_proto", DataType::Binary),
    ])
}

pub fn translate_tx_simple<'a>(tx: &Transaction) -> RgResult<Row<'a>> {
    let time = tx.time()?.clone();
    let transaction_proto = tx.proto_serialize();
    let hash = tx.hash_proto_bytes();

    Ok(Row::new(vec![
        AnyValue::Datetime(time, TimeUnit::Milliseconds, &None),
        AnyValue::BinaryOwned(hash),
        AnyValue::BinaryOwned(transaction_proto),
    ]))
}
