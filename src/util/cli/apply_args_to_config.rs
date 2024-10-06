use redgold_schema::conf::rg_args::RgArgs;
use redgold_schema::config_data::ConfigData;
use redgold_schema::structs::NetworkEnvironment;

pub fn apply_args_initial(rg_args: Box<RgArgs>, config: Box<ConfigData>) -> Box<ConfigData> {
    let mut config = config.clone();

    let mut sd = config.secure.unwrap_or(Default::default());
    sd.path = rg_args.secure_data_path;
    sd.config = rg_args.secure_data_config_path;
    config.secure = Some(sd);

    config.home = rg_args.home;
    config.config = rg_args.config_path;
    config.data = rg_args.data_path;
    config.network = rg_args.network;
    if config.network.is_none() && rg_args.development_mode {
        config.network = Some(NetworkEnvironment::Dev.to_std_string());
    }
    config
}
pub fn apply_args_final(rg_args: Box<RgArgs>, config: Box<ConfigData>) -> Box<ConfigData> {
    let mut config = config.clone();

    config
}