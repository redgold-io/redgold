use std::io::Read;
use std::path::Path;
use wasmer::{Cranelift, ImportObject, imports, Instance, Module, Store, Universal, Value};
use wasmer_compiler_llvm::LLVM;

// //! This is a simple example introducing the core concepts of the Wasmer API.
// //!
// //! You can run the example directly by executing the following in the Wasmer root:
// //!
// //! ```shell
// //! cargo run --example hello-world --release --features "cranelift"
// //! ```
//
// use wasmer::{
//     imports, wat2wasm, Function, FunctionEnv, FunctionEnvMut, Instance, Module, Store,
//     TypedFunction,
// };
// use wasmer_compiler::Universal;
// use wasmer_compiler_cranelift::Cranelift;
//
// fn main() {
//     // First we create a simple Wasm program to use with Wasmer.
//     // We use the WebAssembly text format and use `wasmer::wat2wasm` to compile
//     // it into a WebAssembly binary.
//     //
//     // Most WebAssembly programs come from compiling source code in a high level
//     // language and will already be in the binary format.
//     let wasm_bytes = wat2wasm(
//         br#"
// (module
//   ;; First we define a type with no parameters and no results.
//   (type $no_args_no_rets_t (func (param) (result)))
//   ;; Then we declare that we want to import a function named "env" "say_hello" with
//   ;; that type signature.
//   (import "env" "say_hello" (func $say_hello (type $no_args_no_rets_t)))
//   ;; Finally we create an entrypoint that calls our imported function.
//   (func $run (type $no_args_no_rets_t)
//     (call $say_hello))
//   ;; And mark it as an exported function named "run".
//   (export "run" (func $run)))
// "#,
//     )?;
//
//     // Next we create the `Store`, the top level type in the Wasmer API.
//     //
//     // Note that we don't need to specify the engine/compiler if we want to use
//     // the default provided by Wasmer.
//     // You can use `Store::default()` for that.
//     //
//     // However for the purposes of showing what's happening, we create a compiler
//     // (`Cranelift`) and pass it to an engine (`Universal`). We then pass the engine to
//     // the store and are now ready to compile and run WebAssembly!
//     let mut store = Store::new_with_engine(&Universal::new(Cranelift::default()).engine());
//
//     // We then use our store and Wasm bytes to compile a `Module`.
//     // A `Module` is a compiled WebAssembly module that isn't ready to execute yet.
//     let module = Module::new(&store, wasm_bytes)?;
//
//     // Next we'll set up our `Module` so that we can execute it. First, create
//     // a `FunctionEnv` in which to instantiate our `Module`.
//     let mut context = FunctionEnv::new(&mut store, ());
//
//     // We define a function to act as our "env" "say_hello" function imported in the
//     // Wasm program above.
//     fn say_hello_world(_env: FunctionEnvMut<'_, ()>) {
//         println!("Hello, world!")
//     }
//
//     // We then create an import object so that the `Module`'s imports can be satisfied.
//     let import_object = imports! {
//         // We use the default namespace "env".
//         "env" => {
//             // And call our function "say_hello".
//             "say_hello" => Function::new_native(&mut store, &context, say_hello_world),
//         }
//     };
//
//     // We then use the `Module` and the import object to create an `Instance`.
//     //
//     // An `Instance` is a compiled WebAssembly module that has been set up
//     // and is ready to execute.
//     let instance = Instance::new(&mut store, &module, &import_object)?;
//
//     // We get the `TypedFunction` with no parameters and no results from the instance.
//     //
//     // Recall that the Wasm module exported a function named "run", this is getting
//     // that exported function from the `Instance`.
//     let run_func: TypedFunction<(), ()> = instance.exports.get_typed_function(&mut store, "run")?;
//
//     // Finally, we call our exported Wasm function which will call our "say_hello"
//     // function and return.
//     run_func.call(&mut store)?;
//
// }
//
#[test]
fn test_hello_world() {
    use wasmer::{Store, Module, Instance, Value, imports};

    fn main() -> anyhow::Result<()> {
        let module_wat = r#"
    (module
      (type $t0 (func (param i32) (result i32)))
      (func $add_one (export "add_one") (type $t0) (param $p0 i32) (result i32)
        get_local $p0
        i32.const 1
        i32.add))
    "#;

        let store = Store::default();
        let module = Module::new(&store, &module_wat)?;
        // The module doesn't import anything, so we create an empty import object.
        let import_object = imports! {};
        let instance = Instance::new(&module, &import_object)?;

        let add_one = instance.exports.get_function("add_one")?;
        let result = add_one.call(&[Value::I32(42)])?;
        assert_eq!(result[0], Value::I32(43));

        Ok(())
    }
}
/*
    #[cfg(not(test))]
    fn instance_store() -> Store {
        // use wasmer::Dylib;
        use wasmer::Universal;
        use wasmer_compiler_llvm::LLVM;
        // Store::new(&Dylib::headless().engine())
        Store::new(&Universal::new(LLVM::new()).engine())
    }

    #[cfg(test)]
    fn instance_store() -> Store {
        use wasmer::{Cranelift, Universal};
        Store::new(&Universal::new(Cranelift::new()).engine())
    }
    // We create a headless Universal engine.
    // let engine = LLVM::headless().engine();
    // let mut store = wasmer::Store::new(&engine);
    // let mut store = Store::new(&Universal::new(Cranelift::new()).engine());
    // let mut store = Store::new(&Dylib::headless().engine());
  // Here we go.
    //
    // Deserialize the compiled Wasm module. This code is unsafe
    // because Wasmer can't assert the bytes are valid (see the
    // `wasmer::Module::deserialize`'s documentation to learn
    // more).

    // let module = unsafe { Module::deserialize_from_file(&store, path) }.expect("loaded module");

    // Congrats, the Wasm module has been deserialized! Now let's
    // execute it for the sake of having a complete example.

    // Create an import object. Since our Wasm module didn't declare
    // any imports, it's an empty object.
    // let path = Path::new("/home/$USER/projects/redgold-core/examples/rust_wasm/test_contract_guest.wasm");

 */
use wasmer::Dylib;

fn call_add_one(path: &Path, imports: Option<ImportObject>) {
    let mut store = Store::new(&Universal::new(LLVM::new()).engine());
    use std::io::Read;
    let data = std::fs::read(path).unwrap();
    let module = Module::new(&store, data).expect("m");
    let import_object = imports.unwrap_or(imports! {});
    let instance = Instance::new( &module, &import_object).expect("instance");
    let add_one = instance.exports.get_function("add_one").expect("function");
    let result = add_one.call(&[Value::I32(42)]).expect("call");
    assert_eq!(result[0], Value::I32(43));
}

#[test]
fn test_loading_rust_code() {
    let cd_module = std::env::current_dir().expect("cwd");
    let cd = cd_module.parent().expect("parent");
    println!("{:?}", cd.clone());
    let buf = cd.join("examples/rust_wasm/test_contract_guest.wasm");
    let path = buf.as_path();
    println!("{:?}", path.clone().to_str().expect("a").to_string());
    call_add_one(path, None);
}

use wasmer::{
    FunctionType, Type, Function
};

/*
logString print logInt logOutOfMemory
 */
/// let import_object = imports! {
///     "env" => {
///         "foo" => Function::new_native(&store, foo)
///     },
/// };
///
///
fn log_string(n: i32){
    println!("{:?}", n)
}

fn log_int(n: i32){
    println!("{:?}", n)
}

fn log_oom(){
    println!("out of memory");
}

fn init(i: i32) {

}

// todo: Fix
fn currentTimeMillis() -> f64 {
    0 as f64
}


#[ignore]
#[test]
fn test_loading_java_code() {
    let path = Path::new("/home/$USER/projects/teavm/samples/benchmark/target/generated/wasm/teavm-wasm/classes.wasm");
    let mut store = Store::new(&Universal::new(LLVM::new()).engine());
    // let mut env1 = FunctionEnv::new(&mut store, ());

    use std::io::Read;
    let data = std::fs::read(path).unwrap();
    let module = Module::new(&store, data).expect("m");
    // let import_object = imports.unwrap_or(imports! {});
    let import_object = imports! {
        "teavm" => {
            "logString" => Function::new_native(&store, log_string),
            "logInt" => Function::new_native(&store, log_int),
            "logOutOfMemory" => Function::new_native(&store, log_oom),
            "currentTimeMillis" => Function::new_native(&store, currentTimeMillis)
        },
        "teavmHeapTrace" => {
            "init" => Function::new_native(&store, init)
        }
    };
    // let import_object = imports!{};
    let instance = Instance::new( &module, &import_object).expect("instance");

    // This has crazy behavior
    /*
    I32(283808)
[I32(283808)]
[I32(283796)]

     */
    let add_one = instance.exports.get_function("add_one").expect("function");
    let result = add_one.call(&[Value::I32(42)]).expect("call");
    println!("{:?}", result[0]);
    println!("{:?}", instance.exports.get_function("add_one2").expect("function").call(&[Value::I32(1)]).expect("call"));
    println!("{:?}", instance.exports.get_function("get_one").expect("function").call(&[Value::I32(0)]).expect("call"));

    //assert_eq!(result[0], Value::I32(43));
}
/*

    println!("Calling `add one` function...");
    // The Wasm module exports a function called `sum`.
    // let sum = instance.exports.get_function("sum")?;
    // let results = sum.call(&mut store, &[Value::I32(1), Value::I32(2)])?;

    // println!("Results: {:?}", results);
    // assert_eq!(results.to_vec(), vec![Value::I32(3)]);

 */

/*


    println!("Calling `sum` function...");
    // Let's call the `sum` exported function. The parameters are a
    // slice of `Value`s. The results are a boxed slice of `Value`s.
    let args = [Value::I32(1), Value::I32(2)];
    let result = sum.call(&mut store, &args)?;

    println!("Results: {:?}", result);
    assert_eq!(result.to_vec(), vec![Value::I32(3)]);

    // That was fun. But what if we can get rid of the `Value`s? Well,
    // that's possible with the `TypedFunction` API. The function
    // will use native Rust values.
    //
    // Note that `native` takes 2 generic parameters: `Args` and
    // `Rets`, respectively for the parameters and the results. If
    // those values don't match the exported function signature, an
    // error will be raised.
    let sum_native: TypedFunction<(i32, i32), i32> = sum.native(&mut store)?;

    println!("Calling `sum` function (natively)...");
    // Let's call the `sum` exported function. The parameters are
    // statically typed Rust values of type `i32` and `i32`. The
    // result, in this case particular case, in a unit of type `i32`.
    let result = sum_native.call(&mut store, 3, 4)?;

    println!("Results: {:?}", result);
    assert_eq!(result, 7);


        let module = Module::new(&self.store, contract.code().data())?;

 */