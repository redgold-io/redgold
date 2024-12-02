use redgold_schema::conf::rg_args::RgArgs;
use redgold_schema::config_data::ConfigData;
use redgold_schema::structs::NetworkEnvironment;

pub fn apply_args_initial(rg_args: Box<RgArgs>, config: Box<ConfigData>) -> Box<ConfigData> {
    let mut config = config.clone();

    if rg_args.config_paths.home.is_some() {
        config.home = rg_args.config_paths.home;
    }
    if rg_args.config_paths.config_path.is_some() {
        config.config = rg_args.config_paths.config_path;
    }

    if let Some(n) = rg_args.global_settings.network.as_ref() {
        config.network = Some(n.clone());
    }

    if config.network.is_none() && config.debug.as_ref().and_then(|x| x.develop).unwrap_or(false) {
        config.network = Some(NetworkEnvironment::Dev.to_std_string());
    }
    config
}
pub fn apply_args_final(rg_args: Box<RgArgs>, config: Box<ConfigData>) -> Box<ConfigData> {
    let mut config = config.clone();
    let node = config.node.get_or_insert(Default::default());
    let debug = config.debug.get_or_insert(Default::default());
    let cli = config.cli.get_or_insert(Default::default());


    if let Some(w) = rg_args.global_settings.words.as_ref() {
        node.words = Some(w.clone());
    }
    
    if rg_args.global_settings.offline {
        config.offline = Some(true);
    }

    if let Some(d) = rg_args.debug_args.debug_id.as_ref() {
        debug.id = Some(d.clone());
    }

    if rg_args.cli_settings.cold {
        cli.cold = Some(true);
    }
    if rg_args.cli_settings.airgap {
        cli.airgap = Some(true);
    }
    if rg_args.cli_settings.account.is_some() {
        cli.account = rg_args.cli_settings.account.clone();
    }
    if rg_args.cli_settings.currency.is_some() {
        cli.currency = rg_args.cli_settings.currency.clone();
    }
    if rg_args.cli_settings.path.is_some() {
        cli.path = rg_args.cli_settings.path.clone();
    }
    if rg_args.cli_settings.verbose {
        cli.verbose = Some(true);
    }

    config
}