use extism::{Context, Plugin};


#[test]
fn debug_test() {

    let context = Context::new();
    // let wasm = include_bytes!("code.wasm");
    // let wasm = include_bytes!("../../sdk/test_contract_guest.wasm");
    let wasm = include_bytes!("../../sdk/extism_test.wasm");

    // NOTE: if you encounter an error such as:
    // "Unable to load plugin: unknown import: wasi_snapshot_preview1::fd_write has not been defined"
    // change `false` to `true` in the following function to provide WASI imports to your plugin.
    let mut plugin = Plugin::new(&context, wasm, vec![], false).unwrap();
    let data = plugin.call("count_vowels", "this is a test").unwrap();
    println!("data: {:?}", String::from_utf8(data.to_vec()));
    assert_eq!(data, b"{\"count\": 4}");
}