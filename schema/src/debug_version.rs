use crate::proto_serde::ProtoSerde;
use crate::structs::{DebugVersionChange, DebugVersionChange2};

#[test]
fn test_json_debug() {
    let original = DebugVersionChange{
        field1: Some("asdf".to_string()),
    };

    let ser = serde_json::to_string(&original).unwrap();
    let deser: DebugVersionChange2 = serde_json::from_str(&ser).unwrap();
    assert_eq!(deser.field1, Some("asdf".to_string()));
}

fn test_proto_debug() {
    let original = DebugVersionChange{
        field1: Some("asdf".to_string()),
    };
    let ser = original.proto_serialize();
    let deser = DebugVersionChange2::proto_deserialize(ser).unwrap();
    assert_eq!(deser.field1, Some("asdf".to_string()));
}

