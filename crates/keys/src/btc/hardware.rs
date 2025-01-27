
//
// pub fn new_hardware_wallet(
//     public_key: structs::PublicKey,
//     network: NetworkEnvironment,
//     do_sync: bool
// ) -> Result<Self, ErrorInfo> {
//     let network = if network == NetworkEnvironment::Main {
//         Network::Bitcoin
//     } else {
//         Network::Testnet
//     };
//     let client = Client::new("ssl://electrum.blockstream.info:60002")
//         .error_info("Error building bdk client")?;
//     let client = ElectrumBlockchain::from(client);
//     let database = MemoryDatabase::default();
//     let hex = public_key.hex_or();
//     let descr = format!("wpkh({})", hex);
//     let wallet = Wallet::new(
//         &*descr,
//         Some(&*descr),
//         network,
//         database
//     ).error_info("Error creating BDK wallet")?;
//     // let custom_signer = Arc::new(MultipartySigner::new(public_key.clone()));
//     let mut devices = HWIClient::enumerate()?;
//     if devices.is_empty() {
//         panic!("No devices found!");
//     }
//     let first_device = devices.remove(0)?;
//     let custom_signer = HWISigner::from_device(&first_device, HWIChain::Test)?;
//
//
//     let mut bitcoin_wallet = Self {
//         wallet,
//         public_key,
//         network,
//         psbt: None,
//         transaction_details: None,
//         client,
//         custom_signer: custom_signer.clone(),
//     };
//     // Adding the multiparty signer to the BDK wallet
//     bitcoin_wallet.wallet.add_signer(
//         KeychainKind::External,
//         SignerOrdering(200),
//         custom_signer.clone(),
//     );
//
//     if do_sync {
//         bitcoin_wallet.sync()?;
//     }
//     Ok(bitcoin_wallet)
// }