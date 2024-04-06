use crate::schema::structs::Error;
use crate::util;
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct BitcoindRpcAuth {
    rpc_auth_config_line: String,
    password: String,
}

pub fn generate_username_rpc_auth<S: Into<String>>(username: S) -> Result<BitcoindRpcAuth, Error> {
    // let command = format!("curl -sSL https://raw.githubusercontent.com/bitcoin/bitcoin/master/share/rpcauth/rpcauth.py | python - {}",
    //                       username.into());
    let (stdout, _) = redgold_schema::util::cmd::run_cmd(
        "python3",
        vec!["./src/resources/infra/rpcauth.py", &*username.into()],
    );
    let lines = stdout.split("\n").collect_vec();
    if let Some(x) = lines.clone().get(0) {
        if let Some(y) = lines.get(1) {
            let rpc_auth_config_line = x.to_string();
            let password = y.to_string();
            return Ok(BitcoindRpcAuth {
                rpc_auth_config_line,
                password,
            });
        }
    }
    // TODO: Error code for BTC RPC
    Err(Error::UnknownError)
}
//

#[test]
fn test_username_generation() {
    println!("{:?}", generate_username_rpc_auth("root"));
}
