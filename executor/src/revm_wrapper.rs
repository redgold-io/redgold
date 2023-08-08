use anyhow::{Ok, Result};
use bytes::Bytes;
use ethers_contract::BaseContract;
use ethers_core::abi::{AbiEncode, AbiError, Contract, Function, FunctionExt, parse_abi, parse_abi_str, Token};
use ethers_providers::{Http, Provider};
use revm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{ExecutionResult, Output, TransactTo, B160, U256 as rU256},
    Database, EVM,
};
use std::{str::FromStr, sync::Arc};
use revm::primitives::{AccountInfo, Bytecode, U256};
use revm::primitives::ruint::Uint;
use tokio;

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn tse() -> Result<()> {

    // choose slot of storage that you would like to transact with
    let slot = rU256::from(8);

    // ETH/USDT pair on Uniswap V2
    let pool_address = B160::from_str("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852")?;

    let f = include_str!("./res/HelloWorld.abi");
    let greeter_code = hex::decode(include_str!("./res/HelloWorld.bin")).expect("");
    // let function: Vec<Function> = serde_json::from_str(f).expect("invalid function JSON");
    let c = serde_json::from_str::<Contract>(f).expect("");
    println!("c: {:?}", c);


    // for f in function {
    //     let signature = f.abi_signature();
    //     // BaseContract::from()
    //     println!("signature: {}", signature);
    // }



    // parse_abi_str()
    // generate abi for the calldata from the human readable interface
    let abi = BaseContract::from(
        c.clone()
        // parse_abi(&[
        //     "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
        // ])?
    );
    println!("m {:?}", abi.methods);

    // let fname = "sayHelloWorld";
    let fname = "getReserves2";
    let contract = c.clone();
    let f_actual = contract.function(fname).expect("");
    // encode abi into Bytes
    let encoded = abi.encode(fname, ())?;
    let mut cache_db = CacheDB::new(EmptyDB::default());
    let caller_address = B160::from_str("0x0000000000000000000000000000000000000000")?;
    let balance = U256::try_from(1000000000000000 as i64).expect("");
    let code = Bytecode::new_raw(Bytes::from(greeter_code));
    let acc_info = AccountInfo::new(balance, 0u64, code);
    cache_db.insert_account_info(pool_address, acc_info.clone());
    let mut evm = EVM::new();
    evm.database(cache_db);
    evm.env.tx.caller = caller_address;
    // account you want to transact with
    evm.env.tx.transact_to = TransactTo::Call(pool_address);
    // calldata formed via abigen
    evm.env.tx.data = Bytes::from(hex::decode(hex::encode(&encoded))?);
    // transaction value in wei
    evm.env.tx.value = rU256::try_from(0)?;
    // execute transaction without writing to the DB
    let ref_tx = evm.transact_ref().unwrap();
    // select ExecutionResult struct
    let result = ref_tx.result;

    let value = match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(value) => Some(value),
            _ => None,
        },
        _ => None,
    };
    let v = value.unwrap();

    // let o: Uint<256, 1> = abi.decode_output(fname, v.clone())?;

    let t = f_actual.decode_output(v.clone().as_ref())?;
    for tt in t {
        println!("tt: {:#?}", tt);
    }
    // println!("o: {:#?}", o);
    // decode bytes to reserves + ts via ethers-rs's abi decode
    let (reserve0, reserve1, ts): (u128, u128, u32) =
        abi.decode_output("getReserves2", v)?;

    // Print emulated getReserves() call output
    println!("Reserve0: {:#?}", reserve0);
    println!("Reserve1: {:#?}", reserve1);
    println!("Timestamp: {:#?}", ts);

    Ok(())
}


