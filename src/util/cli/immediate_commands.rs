use clap::Parser;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::{RgArgs, RgTopLevelSubcommand};
use redgold_schema::config_data::ConfigData;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::structs::ErrorInfo;
use crate::util::cli::commands;

pub fn check_run(b: Box<RgTopLevelSubcommand>) -> bool {
    if let RgTopLevelSubcommand::Node(_) = *b {
        false
    } else if let RgTopLevelSubcommand::GUI(_) = *b {
        false
    } else {
        true
    }
}

// returns if aborting remainder.
pub async fn immediate_commands(
    config: &Box<NodeConfig>, subcmd: Option<Box<RgTopLevelSubcommand>>
) -> bool {
    match subcmd {
        Some(subcmd) => {
            if check_run(subcmd.clone()) {
                immediate_commands_inner(subcmd, config).await;
                return true
            }
        }
        None => {
        }
    }
    false
}

// Pre logger commands
pub async fn immediate_commands_inner(
    subcmd: Box<RgTopLevelSubcommand>, config: &Box<NodeConfig>
) {
    let ret = {
        // Use references to avoid cloning
        match &*subcmd {
            RgTopLevelSubcommand::Address(a) => {
                let result = commands::generate_address(a.clone(), &config).map(|_| ());
                // Explicitly drop the reference to a
                drop(a);
                result
            }
            RgTopLevelSubcommand::Send(a) => {
                let result = commands::send(&a, &config).await;
                drop(a);
                result
            }
            RgTopLevelSubcommand::Balance(a) => {
                commands::balance_lookup(&a, config).await
            }
            _ => Ok(())
        }
    };
    ret.expect("immediate command failed");

    drop(subcmd);
    //
    //
    //     }
    //     RgTopLevelSubcommand::Query(a) => {
    //         commands::query(&a, &config).await
    //     }

    //     RgTopLevelSubcommand::Deploy(d) => {
    //         commands::deploy(&d, &config).await.unwrap().abort();
    //         Ok(())
    //     }
    //     RgTopLevelSubcommand::DebugCommand(d) => {
    //         commands::debug_commands(&d, config).await
    //     }
    //     RgTopLevelSubcommand::GenerateConfig(d) => {
    //         let c = ConfigData::generate_user_sample_config();
    //         toml::to_string(&c)
    //             .error_info("Failed to serialize config")
    //             .map(|x| println!("{}", x))
    //     }
    //
    // }

}

// RgTopLevelSubcommand::TestTransaction(test_transaction_cli) => {
//     commands::test_transaction(&test_transaction_cli, &config).await
// }
// RgTopLevelSubcommand::TestBitcoinBalance(_b) => {
//     commands::test_btc_balance(args.get(0).unwrap(), config.network.clone()).await;
//     Ok(())
// }
// RgTopLevelSubcommand::ConvertMetadataXpub(b) => {
//     commands::convert_metadata_xpub(&b.metadata_file).await
// }
// RgTopLevelSubcommand::GenerateWords(m) => {
//     commands::generate_mnemonic(&m)
// },
// RgTopLevelSubcommand::Faucet(a) => {
//     commands::faucet(&a, &config).await
// }
// RgTopLevelSubcommand::AddServer(a) => {
//     commands::add_server(a, &config).await
// }
