use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::{RgArgs, RgTopLevelSubcommand};
use redgold_schema::config_data::ConfigData;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::structs::ErrorInfo;
use crate::util::cli::commands;

// Pre logger commands
pub async fn immediate_commands(
    subcmd: Box<RgTopLevelSubcommand>, config: &Box<NodeConfig>
) -> RgResult<bool> {
    let mut abort = true;
    let res: Result<(), ErrorInfo> = match *subcmd {
        // RgTopLevelSubcommand::GenerateWords(m) => {
        //     commands::generate_mnemonic(&m)
        // },
        RgTopLevelSubcommand::Address(a) => {
            commands::generate_address(a.clone(), &config).map(|_| ())
        }
        RgTopLevelSubcommand::Send(a) => {
            commands::send(&a, &config).await
        }
        RgTopLevelSubcommand::Query(a) => {
            commands::query(&a, &config).await
        }
        // RgTopLevelSubcommand::Faucet(a) => {
        //     commands::faucet(&a, &config).await
        // }
        // RgTopLevelSubcommand::AddServer(a) => {
        //     commands::add_server(a, &config).await
        // }
        RgTopLevelSubcommand::Balance(a) => {
            commands::balance_lookup(&a, config).await
        }
        // RgTopLevelSubcommand::TestTransaction(test_transaction_cli) => {
        //     commands::test_transaction(&test_transaction_cli, &config).await
        // }
        RgTopLevelSubcommand::Deploy(d) => {
            commands::deploy(&d, &config).await.unwrap().abort();
            Ok(())
        }
        // RgTopLevelSubcommand::TestBitcoinBalance(_b) => {
        //     commands::test_btc_balance(args.get(0).unwrap(), config.network.clone()).await;
        //     Ok(())
        // }
        // RgTopLevelSubcommand::ConvertMetadataXpub(b) => {
        //     commands::convert_metadata_xpub(&b.metadata_file).await
        // }
        RgTopLevelSubcommand::DebugCommand(d) => {
            commands::debug_commands(&d, config).await
        }
        RgTopLevelSubcommand::GenerateConfig(d) => {
            let c = ConfigData::generate_user_sample_config();
            toml::to_string(&c)
                .error_info("Failed to serialize config")
                .map(|x| println!("{}", x))
        }
        _ => {
            abort = false;
            Ok(())
        }
    };
    if res.is_err() {
        println!("{}", serde_json::to_string(&res.err().unwrap()).expect("json"));
        abort = true;
    }
    Ok(abort)
}