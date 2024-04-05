// https://docs.rs/ssh/latest/ssh2/
// brew install openssl@1.1

pub mod bitcoind;
pub mod matrix;
pub mod netmaker;
pub mod deploy;
pub mod multiparty_backup;
mod grafana_public_manual_deploy;
//
// use bdk::bitcoin::util::bip32::ExtendedPrivKey;
// use bdk::bitcoin::Network;
// use std::fs;
// use std::fs::File;
// use std::io::prelude::*;
// use std::net::TcpStream;
// use std::path::Path;
// use std::process::id;
// use log::info;
//
// // use ssh2::{Channel, Session};
// use redgold_schema::{ErrorInfoContext, RgResult};
// use redgold_schema::servers::Server;
// use redgold_schema::structs::ErrorInfo;
//
// /*
//
// [Unit]
// Description=Redgold Testnet
//
// [Service]
// ExecStart=/root/redgold_linux
//
// [Install]
// WantedBy=multi-user.target
//
// /etc/systemd/system/redgold-testnet.service
// systemctl daemon-reload
// journalctl -u redgold-testnet.service  -b
//
// systemctl restart redgold-testnet.service
// systemctl status redgold-testnet.service
// scp src/resources/service_scripts/redgold_service_template root@redgold.cash:/etc/systemd/system/redgold-testnet.service
//
// ssh root@redgold.cash '
//  */
//
// pub struct SSH {
//     pub host: String,
//     pub user: Option<String>,
//     pub private_key_path: Option<String>,
//     pub session: Option<Session>
// }
//
// pub struct SSHResult {
//     pub stdout: String,
//     pub stderr: String,
//     pub exit_code: i32
// }
//
// // https://grafana.com/docs/grafana-cloud/quickstart/docker-compose-linux/
// impl SSH {
//
//     pub fn allow_tcp(&mut self, port: u16) -> SSHResult {
//         self.exec(format!("sudo ufw allow proto tcp from any to any port {:?}", port), true)
//     }
//
//     fn session(&mut self) -> Session {
//         if let Some(s) = &self.session {
//             return s.clone();
//         }
//         let tcp = TcpStream::connect(self.host.clone()).unwrap();
//         let mut sess = Session::new().unwrap();
//         sess.set_tcp_stream(tcp);
//         sess.handshake().unwrap();
//
//         let mut agent = sess.agent().unwrap();
//
//         // Connect the agent and request a list of identities
//         agent.connect().unwrap();
//         agent.list_identities().unwrap();
//
//         for identity in agent.identities().unwrap() {
//             println!("{}", identity.comment());
//             let pubkey = identity.blob();
//             println!("public key {}", hex::encode(pubkey));
//         }
//
//         // Try to authenticate with the first identity in the agent.
//         sess.userauth_agent("root").unwrap();
//
//         // Make sure we succeeded
//         assert!(sess.authenticated());
//         // let home_dir = dirs::home_dir().expect("Cannot find home directory");
//         // let buf = home_dir.join(".ssh");
//         // let key_path = buf.join("id_rsa");
//         // let username = self.user.clone().unwrap_or("root".to_string());
//         // let string = key_path.to_str().expect("Cannot find key path").to_string();
//         // let x = self.private_key_path.as_ref().unwrap_or(&string);
//         // let path = Path::new(&x);
//         // println!("SSH with username: {username} and key path: {path:?}");
//         // std::fs::read_to_string(path.clone()).expect("read failure");
//         // sess.userauth_pubkey_file(
//         //     &*username,
//         //     None,
//         //     path,
//         //     None,
//         // )
//         // .unwrap();
//         self.session = Some(sess.clone());
//         return sess;
//     }
//     pub fn verify(&mut self) -> Result<(), ErrorInfo> {
//         self.run("echo test").contains("test")
//             .then(|| Ok(()))
//             .unwrap_or(Err(ErrorInfo::error_info("Cannot verify ssh connection")))
//     }
//     pub fn run(&mut self, cmd: &str) -> String {
//         let sess = self.session();
//         let mut channel = sess.channel_session().unwrap();
//         channel.exec(cmd).unwrap();
//         let mut s = String::new();
//         channel.read_to_string(&mut s).unwrap();
//         println!("Stdout from SSH: {}", s);
//         channel.wait_close().expect("channel closed");
//         let mut stderr = String::new();
//         channel
//             .stderr()
//             .read_to_string(&mut stderr)
//             .expect("read failure on stderr");
//         println!("Stderr from SSH: {}", stderr);
//         println!("Exit code from SSH: {}", channel.exit_status().unwrap());
//         // sess.disconnect(None, "Normal Shutdown", None).unwrap();
//         return s;
//     }
//     pub fn exec<S: Into<String>>(&mut self, cmd: S, print: bool) -> SSHResult {
//         let sess = self.session();
//         let mut channel = sess.channel_session().unwrap();
//         let string = cmd.into();
//         if print {
//             println!("Running command: {}", string.clone());
//         }
//         channel.exec(&*string).unwrap();
//         let mut stdout = String::new();
//         channel.read_to_string(&mut stdout).unwrap();
//         channel.wait_close().expect("channel closed");
//         let mut stderr = String::new();
//         channel
//             .stderr()
//             .read_to_string(&mut stderr)
//             .expect("read failure on stderr");
//         let exit = channel.exit_status().unwrap();
//         if print {
//             println!("Stdout from SSH: {}", stdout);
//             println!("Stderr from SSH: {}", stderr);
//             println!("Exit code from SSH: {}", exit);
//         }
//         // sess.disconnect(None, "Normal Shutdown", None).unwrap();
//         return SSHResult{
//             stdout,
//             stderr,
//             exit_code: exit
//         };
//     }
//
//     pub fn read_channel(channel: &mut Channel) -> Result<String, ErrorInfo>  {
//         let mut result = String::new();
//         loop {
//             // If you plan to use this, be aware that reading 1 byte at a time is terribly
//             // inefficient and should be optimized for your usecase. This is just an example.
//             let available = channel.read_window().available;
//
//             let mut zero_vec = vec![1u8; available as usize];
//             // let mut buffer = [1u8; 1000];
//             let buffer = &mut *zero_vec;
//             let bytes_read = channel.read(&mut buffer[..]).expect("works");
//             let s = String::from_utf8_lossy(&buffer[..bytes_read]);
//             let x = &s;
//             let partial_read = x.clone().to_string();
//             result.push_str(x);
//             println!("Finished partial: {partial_read}");
//
//             // if result.ends_with("]]>]]>") {
//             //     println!("Found netconf 1.0 terminator, breaking read loop");
//             //     break;
//             // }
//             // if result.ends_with("##") {
//             //     println!("Found netconf 1.1 terminator, breaking read loop");
//             //     break;
//             // }
//             if channel.eof() { //bytes_read == 0 ||
//                 println!("Buffer is empty, SSH channel read terminated");
//                 println!("Finished read: {result}");
//                 break;
//             }
//         }
//         Ok(result)
//     }
//
//     pub async fn read_channel_partial< F: Fn(String) -> RgResult<()> + 'static>(channel: &mut Channel, partial: &Box<F>) -> Result<String, ErrorInfo>  {
//         let mut result = String::new();
//         loop {
//             let available = channel.read_window().available;
//             if available > 0 {
//                 let mut zero_vec = vec![1u8; available as usize];
//                 let buffer = &mut *zero_vec;
//                 let bytes_read = channel.read(&mut buffer[..]).expect("works");
//                 let s = String::from_utf8_lossy(&buffer[..bytes_read]);
//                 let x = &s;
//                 let partial_read = x.clone().to_string();
//                 if !partial_read.trim().is_empty() {
//                     result.push_str(x);
//                     partial(partial_read)?;
//                 }
//             } else {
//                 let mut stderr = channel.stderr();
//                 let mut buffer = [0u8; 1024];
//                 let bytes_read = stderr.read(&mut buffer[..]).expect("works");
//                 let s = String::from_utf8_lossy(&buffer[..bytes_read]);
//                 let x = &s;
//                 let partial_read = x.clone().to_string();
//                 if !partial_read.trim().is_empty() {
//                     result.push_str(x);
//                     partial(partial_read)?;
//                 }
//             }
//             if channel.eof() {
//                 break;
//             }
//         }
//         Ok(result)
//     }
//
//     pub fn streaming_exec_channel<S: Into<String>>(&mut self, cmd: S, print: bool) -> Channel {
//         let sess = self.session();
//         let mut channel = sess.channel_session().unwrap();
//         let string = cmd.into();
//         if print {
//             println!("Running command: {}", string.clone());
//         }
//         channel.exec(&*string).unwrap();
//         channel
//     }
//
//
//     pub async fn exes<S: Into<String>, F: Fn(String) -> RgResult<()> + 'static>(&mut self, cmd: S, partial: &Box<F>)
//                                        -> Result<String, ErrorInfo> {
//         self.execs(cmd, true, partial).await
//     }
//
//     pub async fn execs<S: Into<String>,  F: Fn(String) -> RgResult<()> + 'static>(
//         &mut self, cmd: S, print: bool, partial: &Box<F>) -> Result<String, ErrorInfo> {
//         let sess = self.session();
//         let mut channel = sess.channel_session().unwrap();
//         let string = cmd.into();
//         let cmd_format: String = format!("Running command: {}", string.clone());
//         if print {
//             println!("{}", cmd_format.clone());
//         }
//         channel.exec(&*string).unwrap();
//         // channel.stderr()
//         partial(cmd_format.clone())?;
//         let mut result = String::new();
//         result.push_str(&*cmd_format);
//         let exact_result = SSH::read_channel_partial(&mut channel, partial).await?;
//         result.push_str(&*exact_result);
//         Ok(result)
//     }
//
//     pub fn copy<S: Into<String>>(&mut self, contents: S, remote_path: String) {
//         println!("Copying to: {}", remote_path);
//         let contents = contents.into();
//         let path = "tmpfile";
//         fs::remove_file("tmpfile").ok();
//         let mut file = File::create(path).expect("create failed");
//         file.write_all(contents.as_bytes()).expect("write temp file");
//         self.scp("./tmpfile", &*remote_path);
//         fs::remove_file("tmpfile").unwrap();
//     }
//
//     pub async fn copy_p<F: Fn(String) -> RgResult<()> + 'static>(
//         &mut self, contents: impl Into<String>, remote_path: impl Into<String> + Clone,
//         partial: &Box<F>
//     ) -> RgResult<()> {
//         partial(format!("Copying to: {}", remote_path.clone().into().clone()))?;
//         self.exes(format!("rm -f {}", remote_path.clone().into().clone()), partial).await?;
//         self.copy(contents.into(), remote_path.into().clone());
//         Ok(())
//     }
//
//     pub fn scp(&mut self, file: &str, remote_path: &str) {
//         use std::io::prelude::*;
//
//         // Connect to the local SSH server
//         let sess = self.session();
//
//         // TODO: Can we just write the contents instead of tmpfile thing?
//         // Write the file
//         let path1 = Path::new(file);
//         let path2 = Path::new(remote_path);
//         let x = fs::metadata(path1).unwrap().len();
//         let mut remote_file = sess.scp_send(path2, 0o644, x, None).unwrap();
//         let vec = fs::read(path1).unwrap();
//         // let x1 = &*vec;
//         // println!("SCP copy local size + bytes sent: {} + {}", x, x1.len());
//         // remote_file.write(x1).unwrap();
//         for iter in vec.chunks(1024) {
//             remote_file.write(iter).unwrap();
//         }
//         // Close the channel and wait for the whole content to be tranferred
//         remote_file.send_eof().unwrap();
//         remote_file.wait_eof().unwrap();
//         remote_file.close().unwrap();
//         remote_file.wait_close().unwrap();
//         // sess.disconnect(None, "Normal Shutdown", None).unwrap();
//
//     }
//     //
//     // fn daemon_reload(&self) {
//     //     self.run("systemctl daemon-reload");
//     // }
//     //
//     // fn install_host_manager(&self) {
//     //     //self.scp()
//     // }
//     //
//     // // local mac os x
//     // // brew services stop grafana
//     // fn redeploy_grafana(&self) {
//     //     self.run("docker kill grafana");
//     //     self.run("docker rm grafana");
//     //     //export ID=$(id -u) # saves your user id in the ID variable
//     //     //--user $ID
//     //     // TODO: This isn't actually open anywhere but need local secret management tool thing.
//     //     // grafana_secret=4k7ZTyh5a2Rk5gM
//     //     //GF_SECURITY_ADMIN_PASSWORD__FILE
//     //     // https://grafana.com/grafana/dashboards/1860
//     //     // docker exec grafana grafana-cli admin reset-admin-password --homepath /usr/share/grafana admin
//     //     // docker exec --user bitcoin bitcoin-server bitcoin-cli
//     //     self.run(
//     //         "docker run -d --user $(id -u) -p 3000:3000 -e GF_SECURITY_ADMIN_PASSWORD__FILE=/var/lib/grafana/grafana_secret --volume /root/data/grafana:/var/lib/grafana --name grafana grafana/grafana-oss",
//     //     );
//     // }
//     /*
//     Service restart script
//     rm -f /root/redgold-linux
//     wget https://redgold-public.s3.us-west-1.amazonaws.com/release/testnet-latest/redgold_linux -O /root/redgold_linux
//     chmod +x /root/redgold-linux
//      */
//     // fn update_redgold(&self) {
//     //     self.run("service redgold-testnet stop");
//     //     self.run("wget https://redgold-public.s3.us-west-1.amazonaws.com/release/test-latest/redgold_linux -O redgold_linux && chmod +x redgold_linux"
//     //     );
//     //     //        self.run("chmod +x redgold_linux");
//     //     // self.run("service redgold-testnet restart");
//     // }
//
//     // TODO: impl bitcoin ? For all methods, how to capture RPC methods?
//
//
//     pub fn new_ssh<S: Into<String>>(host: S, key_path: Option<S>) -> SSH {
//         let x = key_path
//             .map(|x| x.into());
//         SSH {
//             host: format!("{}:22", host.into()),
//             user: None,
//             private_key_path: x,
//             session: None
//         }
//     }
//
//     pub fn new_ssh2<S: Into<String>>(host: S, key_path: Option<S>, user: Option<S>) -> SSH {
//         let x = key_path
//             .map(|x| x.into());
//         let user = user
//             .map(|x| x.into());
//         SSH {
//             host: format!("{}:22", host.into()),
//             user,
//             private_key_path: x,
//             session: None
//         }
//     }
//     pub fn from_server(server: &Server) -> SSH {
//         SSH{
//             host: format!("{}:22", server.host.clone()),
//             user: server.username.clone(),
//             private_key_path: None,
//             session: None,
//         }
//     }
//
//     pub fn docker_logs(&mut self) {
//         self.exec("docker logs --tail 1000 redgold-predev", true);
//     }
//
// }
//
// // fn deploy_ipfs(ssh: SSH) {
// //     /*
// //
// //     ufw allow 4001
// //     export ipfs_staging=/mnt/md0/ipfs_staging
// //     export ipfs_data=/mnt/md0/ipfs_data
// //     docker run -d --name ipfs_host -v $ipfs_staging:/export -v $ipfs_data:/data/ipfs -p 4001:4001 -p 4001:4001/udp -p 127.0.0.1:8081:8080 -p 127.0.0.1:5001:5001 ipfs/go-ipfs:latest
// //      */
// //     //
// // }
//
// // fn deploy_test(ssh: SSH) {
// //     ssh.scp("./target/debug/redgold", "/root/redgold-debug");
// // }
//
// // https://grafana.com/docs/grafana/latest/installation/docker/
// #[ignore]
// #[test]
// fn debug_ssh() {
//     // let do_run = std::env::var("REDGOLD_LOCAL_DEBUG");
//     // if do_run.is_err() {
//     //     return;
//     // }
//
//     // let mut ssh = SSH::new_ssh("lb.redgold.io", None);
//     // ssh.verify().expect("works");
//
//     // ssh.update_redgold()
//
//     /*
//         services:
//       kibana:
//     image: docker.elastic.co/kibana/kibana-oss:7.8.0
//         volumes:
//           - ./conf/kibana.yml:/usr/share/kibana/config/kibana.yml
//
//     docker run -p 9090:9090 -v ~/.rg/prometheus.yml:/etc/prometheus/prometheus.yml prom/prometheus");
//
//          */
//     // ssh.redeploy_grafana()
//     // deploy_test(ssh);
//     // ssh.run("ls");
//     // ssh.run("mkdir /root/.rg");
//     // ssh.run("mkdir /root/.rg/prometheus_data");
//     // ssh.run("chmod -R 777 /root/.rg/prometheus_data");
//     // ssh.run("useradd -r -m bitcoin");
//     //
//     // ssh.run("apt install docker.io -y");
//     // ssh.run("apt install docker-compose -y");
//     // ssh.run("docker kill grafana");
//     // ssh.run("docker run -d -p 3000:3000 --name grafana grafana/grafana-oss");
//     // ssh.run("docker run -p 9090:9090 -v ~/.rg/prometheus.yml:/etc/prometheus/prometheus.yml prom/prometheus");
//     // ssh.run("docker-compose -f /root/.rg/docker-compose-prometheus.yml up -d");
//     //
//     //     ssh.scp(
//     //         "./docker-compose-prometheus.yml",
//     //         "/root/.rg/docker-compose-prometheus.yml",
//     //     );
//     //     ssh.scp("./prometheus.yml", "/root/.rg/prometheus.yml");
//
//     // ssh.run("docker exec --user bitcoin optimistic_bassi bitcoin-cli -testnet getmininginfo");
// }
//
// /*
//
// docker run -v /home/bitcoin/.bitcoin:/home/bitcoin/.bitcoin -p 18443:18443 -p 18444:18444 -it --rm ruimarinho/bitcoin-core \
//   -printtoconsole \
//   -regtest=1
//
// /
//     Testnet JSON-RPC: 18332
//     P2P: 18333
//
// // This seems to be working
// docker run -v /home/bitcoin/.bitcoin:/home/bitcoin/.bitcoin -p 18332:18332 -p 18333:18333 -it --rm ruimarinho/bitcoin-core --name bitcoin-server \
//   -printtoconsole \
//   -testnet=1
//
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet getmininginfo
// docker exec --user bitcoin optimistic_bassi bitcoin-cli -testnet getmininginfo
// {
//   "blocks": 2140296,
//   "difficulty": 24914342.53835173,
//   "networkhashps": 201792952262618.6,
//   "pooledtx": 8,
//   "chain": "test",
//   "warnings": "Unknown new rules activated (versionbit 28)"
// }
//
//
//
//
// Available disk
// df --output=target,avail | sort -r -k 2 | sed -n 2p | cut -d ' ' -f 2
// Mount dir
// df --output=target,avail | sort -r -k 2 | sed -n 2p | cut -d ' ' -f 1
// mkdir /mnt/md0/.bitcoin
// chown bitcoin:bitcoin /mnt/md0/.bitcoin
// ln -s /mnt/md0/.bitcoin /home/bitcoin/.bitcoin
// chown -R bitcoin:bitcoin /home/bitcoin/.bitcoin
//
// ❯ curl -sSL https://raw.githubusercontent.com/bitcoin/bitcoin/master/share/rpcauth/rpcauth.py | python - <username>
//
// String to be appended to bitcoin.conf:
// rpcauth=foo:7d9ba5ae63c3d4dc30583ff4fe65a67e$9e3634e81c11659e3de036d0bf88f89cd169c1039e6e09607562d54765c649cc
// Your password:
// qDDZdeQ5vw9XXFeVnXT4PZ--tGN2xNjjR4nrtyszZx0=
//
// mount bitcoin directory first.
//
// migrate to this with docker compose ?
//
// https://docs.docker.com/storage/volumes/
// // This seems to be working -- but it also seems to like block current thread?
// docker run -v /home/bitcoin/.bitcoin:/home/bitcoin/.bitcoin --name bitcoin-server -p 18332:18332 -p 18333:18333 -it --rm ruimarinho/bitcoin-core \
//   -printtoconsole \
//   -testnet=1
//
// # this verifies it works
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet getmininginfo
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet createwallet test
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet getnewaddress
//
// root@redgold:~# docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet getnewaddress
// tb1qlxknamcg90userzl5vfv3nr6tvyk0ql2awwe27
// root@redgold:~# docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet getnewaddress
// tb1q86cv3cn5d76a5s96clf6sykhq5hnpgzuqy3xef
//
// https://github.com/BlockchainCommons/Learning-Bitcoin-from-the-Command-Line/blob/master/06_1_Sending_a_Transaction_to_a_Multisig.md
// address1=$(bitcoin-cli getnewaddress)
//
// works
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet -named getaddressinfo address=tb1q86cv3cn5d76a5s96clf6sykhq5hnpgzuqy3xef | jq -r '.pubkey'
// 02b84f45abb02024776aa00938d85542dc77f3a557139b76825e59a367c06b17e3
// ^ pubkey
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet -named getaddressinfo address=tb1qlxknamcg90userzl5vfv3nr6tvyk0ql2awwe27 | jq -r '.pubkey'
// 0229e019ac1fc1ade81a34fdf602437323ded9d1b0b393816192145ed2dd83c637
//
// // sortedmulti
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet -named createmultisig nrequired=1 keys='["02b84f45abb02024776aa00938d85542dc77f3a557139b76825e59a367c06b17e3","0229e019ac1fc1ade81a34fdf602437323ded9d1b0b393816192145ed2dd83c637"]'
//
// {
//   "address": "2NDLcLLJ8V1sKB2YWX8MPtAJWRiaPX6psu8",
//   "redeemScript": "512102b84f45abb02024776aa00938d85542dc77f3a557139b76825e59a367c06b17e3210229e019ac1fc1ade81a34fdf602437323ded9d1b0b393816192145ed2dd83c63752ae",
//   "descriptor": "sh(multi(1,02b84f45abb02024776aa00938d85542dc77f3a557139b76825e59a367c06b17e3,0229e019ac1fc1ade81a34fdf602437323ded9d1b0b393816192145ed2dd83c637))#muerqy99"
// }
//
// docker exec --user bitcoin bitcoin-server bitcoin-cli -testnet dumpwallet
// next part todo:
//
//
// $ utxo_txid=$(bitcoin-cli listunspent | jq -r '.[0] | .txid')
// $ utxo_vout=$(bitcoin-cli listunspent | jq -r '.[0] | .vout')
// $ recipient="2N8MytPW2ih27LctLjn6LfLFZZb1PFSsqBr"
//
// $ rawtxhex=$(bitcoin-cli -named createrawtransaction inputs='''[ { "txid": "'$utxo_txid'", "vout": '$utxo_vout' } ]''' outputs='''{ "'$recipient'": 0.000065}''')
// $ bitcoin-cli -named decoderawtransaction hexstring=$rawtxhex
// {
//   "txid": "b164388854f9701051809eed166d9f6cedba92327e4296bf8a265a5da94f6521",
//   "hash": "b164388854f9701051809eed166d9f6cedba92327e4296bf8a265a5da94f6521",
//   "version": 2,
//   "size": 83,
//   "vsize": 83,
//   "weight": 332,
//   "locktime": 0,
//   "vin": [
//     {
//       "txid": "c6de60427b28d8ec8102e49771e5d0348fc3ef6a5bf02eb864ec745105a6951b",
//       "vout": 0,
//       "scriptSig": {
//         "asm": "",
//         "hex": ""
//       },
//       "sequence": 4294967295
//     }
//   ],
//   "vout": [
//     {
//       "value": 0.00006500,
//       "n": 0,
//       "scriptPubKey": {
//         "asm": "OP_HASH160 a5d106eb8ee51b23cf60d8bd98bc285695f233f3 OP_EQUAL",
//         "hex": "a914a5d106eb8ee51b23cf60d8bd98bc285695f233f387",
//         "reqSigs": 1,
//         "type": "scripthash",
//         "addresses": [
//           "2N8MytPW2ih27LctLjn6LfLFZZb1PFSsqBr"
//         ]
//       }
//     }
//   ]
// }
//
// $ signedtx=$(bitcoin-cli -named signrawtransactionwithwallet hexstring=$rawtxhex | jq -r '.hex')
// $ bitcoin-cli -named sendrawtransaction hexstring=$signedtx
// b164388854f9701051809eed166d9f6cedba92327e4296bf8a265a5da94f6521
//
// ^ not yet implemented or tested.
//
// how to get a wifkey
//
// bitcoin-cli sethdseed true "wifkey"
//
//
// */
// //
// // #[test]
// // fn test_wif_key_dump() {
// //     let sk = Wallet::default().seed;
// //
// //     let epk = ExtendedPrivKey::new_master(Network::Testnet, &*sk.0).expect("key");
// //     //
// //     // let pk = bitcoin::util::key::PrivateKey {
// //     //     compressed: false,
// //     //     network: Network::Testnet,
// //     //     key: sk,
// //     // };
// //     let wif = epk.private_key.to_wif();
// //     println!("WIF key: {}", wif);
// // }
