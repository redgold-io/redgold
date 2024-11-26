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
    if rg_args.offline {
        config.offline = Some(true);
    }
    if rg_args.s3_backup_bucket.is_some() {
        let mut er = config.external.clone().unwrap_or(Default::default());
        er.s3_backup_bucket = rg_args.s3_backup_bucket.clone();
        config.external = Some(er);
    }
    if rg_args.aws_access_key_id.is_some() {
        let mut er = config.keys.clone().unwrap_or(Default::default());
        er.aws_access = rg_args.aws_access_key_id.clone();
        config.keys = Some(er);
    }
    if rg_args.aws_secret_access_key.is_some() {
        let mut er = config.keys.clone().unwrap_or(Default::default());
        er.aws_secret = rg_args.aws_secret_access_key.clone();
        config.keys = Some(er);
    }
    if rg_args.development_mode {
        let mut dbg = config.debug.clone().unwrap_or(Default::default());
        dbg.develop = Some(true);
        config.debug = Some(dbg);
    }
    if rg_args.development_mode_main {
        let mut dbg = config.debug.clone().unwrap_or(Default::default());
        dbg.developer = Some(true);
        config.debug = Some(dbg);
    }
    if rg_args.debug_id.is_some() {
        let mut dbg = config.debug.clone().unwrap_or(Default::default());
        dbg.id = rg_args.debug_id.clone();
        config.debug = Some(dbg);
    }
    if rg_args.from_email.is_some() {
        let mut dbg = config.email.clone().unwrap_or(Default::default());
        dbg.from = rg_args.from_email.clone();
        config.email = Some(dbg);
    }
    if rg_args.to_email.is_some() {
        let mut dbg = config.email.clone().unwrap_or(Default::default());
        dbg.to = rg_args.to_email.clone();
        config.email = Some(dbg);
    }

    if rg_args.enable_party_mode {
        let mut dbg = config.party.clone().unwrap_or(Default::default());
        dbg.enable = Some(true);
        config.party = Some(dbg);
    }

    config
}