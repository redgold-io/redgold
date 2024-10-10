use redgold_schema::conf::rg_args::RgArgs;
use redgold_schema::config_data::ConfigData;
use redgold_schema::structs::NetworkEnvironment;

pub fn apply_args_initial(rg_args: Box<RgArgs>, config: Box<ConfigData>) -> Box<ConfigData> {
    let mut config = config.clone();

    let mut sd = config.secure.unwrap_or(Default::default());
    if let Some(sdp) = rg_args.secure_data_path.as_ref() {
        sd.path = Some(sdp.clone());
    }
    if let Some(smh) = rg_args.secure_data_config_path.as_ref() {
        sd.config = Some(smh.clone());
    }
    config.secure = Some(sd);

    if rg_args.home.is_some() {
        config.home = rg_args.home;
    }
    if rg_args.config_path.is_some() {
        config.config = rg_args.config_path;
    }
    if rg_args.data_path.is_some() {
        config.data = rg_args.data_path;
    }
    if rg_args.network.is_some() {
        config.network = rg_args.network;
    }

    if config.network.is_none() && rg_args.development_mode {
        config.network = Some(NetworkEnvironment::Dev.to_std_string());
    }
    config
}
pub fn apply_args_final(rg_args: Box<RgArgs>, config: Box<ConfigData>) -> Box<ConfigData> {
    let mut config = config.clone();

    config
}