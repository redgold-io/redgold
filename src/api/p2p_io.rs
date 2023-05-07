

use std::error::Error;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use async_std::io;
use bitcoin::secp256k1::SecretKey;
use futures::channel::mpsc;
use futures::channel::mpsc::{Receiver, Sender};

use futures::prelude::*;
use libp2p::core::{Multiaddr, PeerId};
use log::{error, info};
use metrics::increment_counter;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use rgnetwork::{Client, Event};

use crate::api::p2p_io::rgnetwork::{EventLoop, NetworkReturn};
use crate::core::internal_message::TransactionMessage;
use crate::core::peer_rx_event_handler::PeerRxEventHandler;
use crate::core::relay::Relay;
use crate::genesis::create_genesis_transaction;
use crate::schema::structs;
use crate::schema::structs::Request;
use crate::schema::structs::Response;
use crate::schema::structs::ResponseMetadata;
use crate::schema::structs::{ErrorInfo, GossipTransactionRequest};
use crate::schema::WithMetadataHashable;
use crate::util;
use crate::util::to_libp2p_kp;

#[derive(Clone)]
pub struct P2P {
    pub client: Client,
    pub address: Multiaddr,
    pub peer_id: PeerId,
}

impl P2P {
    async fn event_process(
        mut network_events: Receiver<Event>,
        mut network_client: Client,
        relay: Relay,
        mut p2p_rx_s: futures::channel::mpsc::Sender<Event>,
        rt: Arc<Runtime>
    ) -> Result<(), ErrorInfo> {
        loop {
            match network_events.next().await {
                Some(e) => {
                    increment_counter!("redgold.libp2p.inbound_request");
                    // Move all this to inbound handler
                    // start an async thread here which sends to another channel
                    // then responds on the interior here with a response if required?
                    // OR use a mpsc channel with a runtime? OR use an ARC?
                    // info!("Received libp2p inbound request: {}", serde_json::to_string(&request.clone())
                    //     .unwrap_or("Request json ser fail".into())
                    // );

                    let mut c2 = network_client.clone();
                    let r = relay.clone();
                    // root cause is  this needs to be non-blocking.
                    let rt2 = rt.clone();
                    // TODO: Handle this with the fut thing
                    // rt.clone().spawn(async move {
                    //     let send_result = libp2phandle_inbound(r, e, c2, rt2).await;
                    //     if let Err(e) = send_result {
                    //         error!("Error handling inbound libp2p request: {}", serde_json::to_string(&e).unwrap());
                    //     }
                    // });

                    //


                    //
                    // let send_result = p2p_rx_s
                    //     .send(rgnetwork::Event::InboundRequest {
                    //         request,
                    //         peer,
                    //         channel,
                    //         remote_address
                    //     })
                    //     .await;
                    // if send_result.is_err() {
                    //     let e = format!("Send to p2p rx channel failed: {}", send_result.unwrap_err().to_string());
                    //     log::error!("{}", e);
                    // }
                }
                None => {}
            }
        }
    }

    pub fn event_channel() -> (futures::channel::mpsc::Sender<Event>, Receiver<Event>) {
        futures::channel::mpsc::channel::<Event>(1000)
    }
    //
    // pub async fn new_async(
    //     relay: Relay,
    //     runtime: Arc<Runtime>,
    //     p2p_rx_s: mpsc::Sender<Event>,
    // ) -> Result<
    //     (
    //         P2P,
    //         P2PRunner, // fn() -> impl Future<Output = ()>
    //                    // impl Fn() -> (dyn Future<Output = ()> + 'static),
    //     ),
    //     ErrorInfo,
    // > {
    //     let port: u16 = relay.node_config.p2p_port();
    //
    //     let sk: &SecretKey = &relay.node_config.wallet().transport_key().secret_key;
    //
    //     let NetworkReturn {
    //         mut client,
    //         event_receiver,
    //         event_loop,
    //         peer_id,
    //     } = rgnetwork::new(to_libp2p_kp(&sk), relay.clone())
    //         .await
    //         .map_err(|e| {
    //             error_message(
    //                 structs::Error::UnknownError,
    //                 format!("p2p network initialization failed: {}", e.to_string()),
    //             )
    //         })?;
    //
    //     let bind_addr = format!("/ip4/0.0.0.0/tcp/{}", port);
    //     let address_str_external = format!("/ip4/127.0.0.1/tcp/{}", port);
    //     let address: Multiaddr = bind_addr.parse().expect("bind address unparseable");
    //     let c2 = client.clone();
    //     client
    //         .clone()
    //         .start_listening(address)
    //         .await
    //         .expect("Failed to start p2p address listener");
    //
    //     let p2p = P2P {
    //         client,
    //         address: address_str_external.parse().unwrap(),
    //         peer_id,
    //     };
    //
    //     let runner = P2PRunner {
    //         relay,
    //         client: c2,
    //         event_loop,
    //         p2p_rx_s,
    //         event_receiver,
    //     };
    //
    //     Ok((p2p, runner))
    // }

    // pub fn new(relay: Relay, runtime: Arc<Runtime>, p2p_rx_s: mpsc::Sender<Event>) -> (P2P, Vec<JoinHandle<Result<(), ErrorInfo>>>) {
    //     let port: u16 = relay.node_config.p2p_port();
    //
    //     // TODO
    //     /*
    // = note: expected reference `&bitcoin::secp256k1::SecretKey`
    //            found reference `&secp256k1::key::SecretKey`
    // = note: perhaps two different versions of crate `secp256k1` are being used?
    //      */
    //     let sk: &SecretKey = &relay.node_config.wallet().transport_key().secret_key;
    //
    //     let NetworkReturn {
    //         mut client,
    //         event_receiver,
    //         event_loop,
    //         peer_id,
    //     } = runtime
    //         .block_on(rgnetwork::new(to_libp2p_kp(&sk), relay.clone()))
    //         .unwrap();
    //     let c2 = client.clone();
    //
    //     let _event_loop = runtime.spawn(event_loop.run());
    //     let bind_addr = format!("/ip4/0.0.0.0/tcp/{}", port);
    //     let address_str_external = format!("/ip4/{}/tcp/{}", relay.node_config.external_ip, port);
    //     let address: Multiaddr = bind_addr.parse().unwrap();
    //     runtime
    //         .block_on(client.start_listening(address))
    //         .expect("Failed to start p2p address listener");
    //
    //     let p2p = P2P {
    //         client,
    //         address: address_str_external.parse().unwrap(),
    //         peer_id,
    //     };
    //     let jh = runtime.spawn(P2P::event_process(event_receiver, c2, relay, p2p_rx_s, runtime.clone()));
    //     (p2p, vec![_event_loop, jh])
    // }
}
//
// pub struct P2PRunner {
//     relay: Relay,
//     client: Client,
//     event_loop: EventLoop,
//     p2p_rx_s: mpsc::Sender<Event>,
//     event_receiver: Receiver<Event>,
// }
//
// impl P2PRunner {
//
//     async fn debug_process(
//         mut network_events: Receiver<Event>,
//         mut network_client: Client,
//         relay: Relay,
//         mut p2p_rx_s: futures::channel::mpsc::Sender<Event>,
//         mut event_loop: EventLoop
//     ) {
//         loop {
//             futures::select! {
//             _ = P2P::process_event(
//                 &mut network_events,
//                 &mut network_client,
//                 relay.clone(),
//                 &mut p2p_rx_s,
//             ).fuse() => (),
//                 _ = event_loop.event_loop_select().fuse() => ()
//         }
//         }
//     }
//
//     pub async fn run(mut self) {
//         let ep = P2P::process_event(
//             &mut self.event_receiver,
//             &mut self.client,
//             self.relay,
//             &mut self.p2p_rx_s,
//         );
//         let el = self.event_loop.event_loop_select();
//         futures::select! {
//             _ = el.fuse() => (),
//             _ = ep.fuse() => ()
//         };
//     }
//     // pub async fn run_loop(&self) {
//     //     loop {
//     //         &self.run().await;
//     //         ()
//     //     }
//     // }
// }

// https://users.rust-lang.org/t/should-i-use-async-or-use-a-separate-thread/22770/3
// #[tokio::test]

// TODO: Re-enable this test later, message passing is not working properly
//
// #[test]
// fn test_p2p_messages() -> Result<(), Box<dyn Error>> {
//     // env_logger::init();
//     util::init_logger().ok();
//     let rt = crate::util::runtimes::build_runtime(10, "p2p".to_string());
//
//     let mut relay1 = rt.block_on(Relay::default());
//     let mut relay2 = rt.block_on(Relay::default());
//     relay1.node_config.p2p_port = Some(4500);
//     relay1.node_config.mnemonic_words =
//         redgold_schema::util::mnemonic_builder::from_str_rounds("0", 0).to_string();
//     relay2.node_config.p2p_port = Some(4501);
//     relay2.node_config.mnemonic_words =
//         redgold_schema::util::mnemonic_builder::from_str_rounds("1", 0).to_string();
//     let (es,  mut p2p_rx_r) = P2P::event_channel();
//     let (mut node1, mut jh1) = P2P::new(relay1, rt.clone(), es);
//     // let tx_rx = relay2.transaction.receiver.clone();
//     let (es2, p2p_rx_r2) = P2P::event_channel();
//     let (mut node2, mut jh2) = P2P::new(relay2.clone(), rt.clone(), es2);
//
//     let eh1 = PeerRxEventHandler::new(
//         node2.client.clone(),
//         p2p_rx_r,
//         relay2.clone(),
//         rt.clone()
//     );
//
//     let eh2 = PeerRxEventHandler::new(
//         node2.client.clone(),
//         p2p_rx_r2,
//         relay2.clone(),
//         rt.clone()
//     );
//
//     sleep(Duration::from_secs(5));
//
//
//     rt.block_on(node1.client.dial(node2.peer_id, node2.address.clone()))
//         .expect("start");
//     // rt.block_on(node2.client.dial(node1.peer_id, node1.address))
//     //     .expect("start");
//
//     let request = Request::empty().about();
//     let response = rt.block_on(node1.client.send_request(node2.peer_id,  request.clone()));
//     // let response = rt.block_on(node1.client.dial_and_send(node2.peer_id, node2.address.clone(), request.clone()));
//     info!("Response : {:?}", response);
//
//     sleep(Duration::from_secs(60));
//
//     let response = rt.block_on(node1.client.dial_and_send(node2.peer_id, node2.address.clone(), request.clone()));
//     info!("Response : {:?}", response);
//
//     sleep(Duration::from_secs(60));
//
//     // let transaction = create_genesis_transaction();
//     //
//     // let response = rt.block_on(node1.client.send_request(
//     //     node2.peer_id,
//     //     Request {
//     //         gossip_transaction_request: Some(GossipTransactionRequest {
//     //             transaction: Some(transaction),
//     //         }),
//     //         gossip_observation_request: None,
//     //         resolve_hash_request: None,
//     //         download_request: None,
//     //         about_node_request: None
//     //     },
//     // ))
//     // .expect("block");
//
//     //sleep(Duration::new(10, 0));
//     Ok(())
// }

// https://users.rust-lang.org/t/should-i-use-async-or-use-a-separate-thread/22770/3
// #[tokio::test]
// #[test]
// fn test_p2p_messages_async() -> Result<(), Box<dyn Error>> {
//     // env_logger::init();
//     let rt = crate::util::runtimes::build_runtime(10, "p2p".to_string());
//
//     let mut relay1 = rt.block_on(Relay::default());
//     let mut relay2 = rt.block_on(Relay::default());
//     relay1.node_config.p2p_port = Some(4500);
//     relay1.node_config.mnemonic_words =
//         crate::util::mnemonic_builder::from_str_rounds("0", 0).to_string();
//     relay2.node_config.p2p_port = Some(4501);
//     relay2.node_config.mnemonic_words =
//         crate::util::mnemonic_builder::from_str_rounds("1", 0).to_string();
//     let (es, _) = P2P::event_channel();
//     let (mut node1, mut node1run) = rt
//         .block_on(P2P::new_async(relay1, rt.clone(), es))
//         .expect("Works");
//     let tx_rx = relay2.transaction.receiver.clone();
//     let (es2, _) = P2P::event_channel();
//     let (mut node2, mut node2run) = rt
//         .block_on(P2P::new_async(relay2, rt.clone(), es2))
//         .expect("Works");
//
//     rt.spawn(async move {
//         loop {
//             node1run.run().await
//         }
//     });
//
//     rt.spawn(async move {
//         loop {
//             node2run.run().await
//         }
//     });
//
//     rt.block_on(node1.client.dial(node2.peer_id, node2.address))
//         .expect("start");
//     rt.block_on(node2.client.dial(node1.peer_id, node1.address))
//         .expect("start");
//     let transaction = create_genesis_transaction();
//     rt.block_on(node1.client.send_request(
//         node2.peer_id,
//         Request {
//             gossip_transaction_request: Some(GossipTransactionRequest {
//                 transaction: Some(transaction),
//             }),
//             gossip_observation_request: None,
//             resolve_hash_request: None,
//             download_request: None,
//         },
//     ))
//     .expect("block");
//
//     let t2 = tx_rx.recv().expect("start");
//
//     assert_eq!(
//         create_genesis_transaction().hash_hex(),
//         t2.transaction.hash_hex()
//     );
//
//     //sleep(Duration::new(10, 0));
//     Ok(())
// }

/// The network module, encapsulating all network related logic.
pub mod rgnetwork {
    use std::collections::{HashMap, HashSet};
    use std::iter;

    use async_trait::async_trait;
    use futures::channel::{mpsc, oneshot};
    use futures::AsyncWriteExt;
    use libp2p::core::either::EitherError;
    use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
    use libp2p::identity::Keypair;
    use libp2p::kad::record::store::MemoryStore;
    use libp2p::kad::{GetProvidersOk, Kademlia, KademliaEvent, QueryId, QueryResult};
    use libp2p::multiaddr::Protocol;
    use libp2p::request_response::{ProtocolSupport, RequestId, RequestResponse, RequestResponseCodec, RequestResponseEvent, RequestResponseMessage, ResponseChannel};
    use libp2p::swarm::{ConnectionHandlerUpgrErr, SwarmBuilder, SwarmEvent};
    use libp2p::{NetworkBehaviour, Swarm};
    use prost::Message;

    use crate::schema::structs::{Request, Response};
    use crate::util::Short;

    use super::*;

    pub struct NetworkReturn {
        pub client: Client,
        pub event_receiver: Receiver<Event>,
        pub event_loop: EventLoop,
        pub peer_id: PeerId,
    }
    /// Creates the network components, namely:
    ///
    /// - The network client to interact with the network layer from anywhere
    ///   within your application.
    ///
    /// - The network event stream, e.g. for incoming requests.
    ///
    /// - The network task driving the network itself.
    pub async fn new(id_keys: Keypair, relay: Relay) -> Result<NetworkReturn, Box<dyn Error>> {
        // // Create a public/private key pair, either random or based on a seed.
        // let id_keys = match secret_key_seed {
        //     Some(seed) => {
        //         let mut bytes = [0u8; 32];
        //         bytes[0] = seed;
        //         let secret_key = ed25519::SecretKey::from_bytes(&mut bytes).expect(
        //             "this returns `Err` only if the length is wrong; the length is correct; qed",
        //         );
        //         identity::Keypair::Ed25519(secret_key.into())
        //     }
        //     None => identity::Keypair::generate_ed25519(),
        // };
        let peer_id = id_keys.public().to_peer_id();

        /*

        // Set up an encrypted TCP Transport over the Mplex
        // This is test transport (memory).
        let noise_keys = libp2p_noise::Keypair::<libp2p_noise::X25519Spec>::new().into_authentic(&local_key).unwrap();
        let transport = MemoryTransport::default()
                   .upgrade(libp2p_core::upgrade::Version::V1)
                   .authenticate(libp2p_noise::NoiseConfig::xx(noise_keys).into_authenticated())
                   .multiplex(libp2p_mplex::MplexConfig::new())
                   .boxed();

        // Create a Gossipsub topic
        let topic = libp2p_gossipsub::IdentTopic::new("example");

        // Set the message authenticity - How we expect to publish messages
        // Here we expect the publisher to sign the message with their key.
        let message_authenticity = MessageAuthenticity::Signed(local_key);

        // Create a Swarm to manage peers and events
        let mut swarm = {
            // set default parameters for gossipsub
            let gossipsub_config = libp2p_gossipsub::GossipsubConfig::default();
            // build a gossipsub network behaviour
            let mut gossipsub: libp2p_gossipsub::Gossipsub =
                libp2p_gossipsub::Gossipsub::new(message_authenticity, gossipsub_config).unwrap();
            // subscribe to the topic
            gossipsub.subscribe(&topic);
            // create the swarm
            libp2p_swarm::Swarm::new(
                transport,
                gossipsub,
                local_peer_id,
            )
        };

                 */
        // Build the Swarm, connecting the lower layer transport logic with the
        // higher layer network behaviour logic.

        // let tcp_transport = TcpTransport::default();

        let swarm = SwarmBuilder::new(
            libp2p::development_transport(id_keys).await?,
            ComposedBehaviour {
                kademlia: Kademlia::new(peer_id, MemoryStore::new(peer_id)),
                request_response: RequestResponse::new(
                    RequestResponseExchangeCodec(),
                    iter::once((PeerExchangeProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
            },
            peer_id,
        )
        .build();

        let (command_sender, command_receiver) = mpsc::channel(0);
        let (event_sender, event_receiver) = mpsc::channel(0);

        Ok(NetworkReturn {
            client: Client {
                sender: command_sender,
            },
            event_receiver: event_receiver,
            event_loop: EventLoop::new(swarm, command_receiver, event_sender, relay),
            peer_id,
        })
    }

    #[derive(Clone)]
    pub struct Client {
        pub sender: mpsc::Sender<Command>,
    }

    impl Client {
        /// Listen for incoming connections on the given address.
        pub async fn start_listening(
            &mut self,
            addr: Multiaddr,
        ) -> Result<(), Box<dyn Error + Send>> {
            let (sender, receiver) = oneshot::channel();
            self.sender
                .send(Command::StartListening { addr, sender })
                .await
                .expect("Command receiver not to be dropped.");
            receiver.await.expect("Sender not to be dropped.")
        }

        pub async fn dial_and_send(&mut self, peer_id: PeerId, peer_addr: Multiaddr, request: Request) -> Result<Response, ErrorInfo> {
            self.dial(peer_id, peer_addr.clone()).await.map_err(|e| ErrorInfo::error_info(e.to_string()))?;
            info!("Dial success to {:?} on addr: {:?} with request: {:?}", peer_id, peer_addr.clone(), request.clone());
            self.send_request(peer_id, request).await
                .map_err(|e| ErrorInfo::error_info(
                    format!("libp2p client send request failure {}", e.to_string())))
        }

        /// Dial the given peer at the given address.
        pub async fn dial(
            &mut self,
            peer_id: PeerId,
            peer_addr: Multiaddr,
        ) -> Result<(), Box<dyn Error + Send>> {
            let (sender, receiver) = oneshot::channel();
            self.sender
                .send(Command::Dial {
                    peer_id,
                    peer_addr,
                    sender,
                })
                .await
                .expect("Command receiver not to be dropped.");
            // tokio::time::timeout(Duration::from_secs(5), receiver)
            receiver
                .await
                // .unwrap()
                //
                //  .timeout()
                //.timeout(Duration::from_secs(5))
                .expect("Sender not to be dropped.")
        }

        // /// Advertise the local node as the provider of the given file on the DHT.
        // pub async fn start_providing(&mut self, file_name: String) {
        //     let (sender, receiver) = oneshot::channel();
        //     self.sender
        //         .send(Command::StartProviding { file_name, sender })
        //         .await
        //         .expect("Command receiver not to be dropped.");
        //     receiver.await.expect("Sender not to be dropped.");
        // }
        //
        // /// Find the providers for the given file on the DHT.
        // pub async fn get_providers(&mut self, file_name: String) -> HashSet<PeerId> {
        //     let (sender, receiver) = oneshot::channel();
        //     self.sender
        //         .send(Command::GetProviders { file_name, sender })
        //         .await
        //         .expect("Command receiver not to be dropped.");
        //     receiver.await.expect("Sender not to be dropped.")
        // }

        /// Request the content of the given file from the given peer.
        pub async fn send_request(
            &mut self,
            peer: PeerId,
            request: Request,
        ) -> Result<Response, Box<dyn Error + Send>> {
            let (sender, receiver) = oneshot::channel();
            self.sender
                .send(Command::SendRequest {
                    request,
                    peer,
                    sender,
                })
                .await
                .expect("Command receiver not to be dropped.");
            receiver.await.expect("Sender not be dropped.")
        }

        /// Respond with the provided file content to the given request.
        pub async fn respond(
            &mut self,
            response: Response,
            channel: ResponseChannel<PeerResponse>,
        ) {
            self.sender
                .send(Command::Respond { response, channel })
                .await
                .expect("Command receiver not to be dropped.");
        }
    }

    #[allow(dead_code)]
    pub struct EventLoop {
        swarm: Swarm<ComposedBehaviour>,
        command_receiver: mpsc::Receiver<Command>,
        event_sender: mpsc::Sender<Event>,
        pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
        pending_start_providing: HashMap<QueryId, oneshot::Sender<()>>,
        pending_get_providers: HashMap<QueryId, oneshot::Sender<HashSet<PeerId>>>,
        pending_requests:
            HashMap<RequestId, oneshot::Sender<Result<Response, Box<dyn Error + Send>>>>,
        relay: Relay,
        active_connections: HashSet<PeerId>,
        address_lookup: HashMap<PeerId, Multiaddr>
    }

    impl EventLoop {
        fn new(
            swarm: Swarm<ComposedBehaviour>,
            command_receiver: mpsc::Receiver<Command>,
            event_sender: mpsc::Sender<Event>,
            relay: Relay,
        ) -> Self {
            Self {
                swarm,
                command_receiver,
                event_sender,
                pending_dial: Default::default(),
                pending_start_providing: Default::default(),
                pending_get_providers: Default::default(),
                pending_requests: Default::default(),
                relay,
                active_connections: HashSet::new(),
                address_lookup: Default::default()
            }
        }

        pub async fn run(mut self) -> Result<(), ErrorInfo> {
            loop {
                futures::select! {
                    event = self.swarm.next() => self.handle_event(event.expect("Swarm stream to be infinite.")).await  ,
                    command = self.command_receiver.next() => match command {
                        Some(c) => self.handle_command(c).await,
                        // Command channel closed, thus shutting down the network event loop.
                        None =>  return Err(ErrorInfo::error_info("Command channel closed libp2p event loop")),
                    },
                }
            }
        }

        pub async fn event_loop_select(mut self) {
            futures::select! {
                event = self.swarm.next() => self.handle_event(event.expect("Swarm stream to be infinite.")).await  ,
                command = self.command_receiver.next() => match command {
                    Some(c) => self.handle_command(c).await,
                    // Command channel closed, thus shutting down the network event loop.
                    None =>  return,
                },
            }
        }

        async fn handle_event(
            &mut self,
            event: SwarmEvent<
                ComposedEvent,
                EitherError<ConnectionHandlerUpgrErr<io::Error>, io::Error>,
            >,
        ) {
            match event {
                SwarmEvent::Behaviour(ComposedEvent::Kademlia(
                    KademliaEvent::OutboundQueryCompleted {
                        id,
                        result: QueryResult::StartProviding(_),
                        ..
                    },
                )) => {
                    let sender: oneshot::Sender<()> = self
                        .pending_start_providing
                        .remove(&id)
                        .expect("Completed query to be previously pending.");
                    let _ = sender.send(());
                }
                SwarmEvent::Behaviour(ComposedEvent::Kademlia(
                    KademliaEvent::OutboundQueryCompleted {
                        id,
                        result: QueryResult::GetProviders(Ok(GetProvidersOk { providers, .. })),
                        ..
                    },
                )) => {
                    let _ = self
                        .pending_get_providers
                        .remove(&id)
                        .expect("Completed query to be previously pending.")
                        .send(providers);
                }
                SwarmEvent::Behaviour(ComposedEvent::Kademlia(_)) => {}
                SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
                    RequestResponseEvent::Message { peer, message },
                )) => match message {
                    RequestResponseMessage::Request {
                        request, channel, ..
                    } => {
                        info!(
                            "{:?} received request, sending to event sender",
                            self.swarm.local_peer_id()
                        );
                        self.event_sender
                            .send(Event::InboundRequest {
                                request: request.0,
                                peer,
                                channel,
                                remote_address: self.address_lookup.get(&peer).unwrap().clone()
                            })
                            .await
                            .expect("Event receiver not to be dropped.");
                    }
                    RequestResponseMessage::Response {
                        request_id,
                        response,
                    } => {
                        info!("Received response: {:?}", self.swarm.local_peer_id());
                        let _ = self
                            .pending_requests
                            .remove(&request_id)
                            .expect("Request to still be pending.")
                            .send(Ok(response.0));
                    }
                },
                SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
                    RequestResponseEvent::OutboundFailure {
                        request_id, error, ..
                    },
                )) => {
                    let _ = self
                        .pending_requests
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Err(Box::new(error)));
                }
                SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
                    RequestResponseEvent::ResponseSent { .. },
                )) => {}
                SwarmEvent::NewListenAddr { address, .. } => {
                    let local_peer_id = *self.swarm.local_peer_id();
                    info!(
                        "LOCAL_ID={} LISTEN_ADDR={}",
                        local_peer_id.short_id(),
                        address
                            .with(Protocol::P2p(local_peer_id.into()))
                            .to_string()
                    );
                }
                SwarmEvent::IncomingConnection {
                    local_addr,
                    send_back_addr,
                } => {
                    info!(
                        "IncomingConnection LOCAL_ADDR={} SEND_BACK_ADDR={} LOCAL_ID={}",
                        local_addr,
                        send_back_addr,
                        self.swarm.local_peer_id().short_id()
                    );
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    endpoint,
                    num_established,
                    .. //concurrent_dial_errors,
                } => {
                    if endpoint.is_dialer() {
                        if let Some(sender) = self.pending_dial.remove(&peer_id) {
                            let _ = sender.send(Ok(()));
                        }
                    }
                    //self.swarm.disconnect_peer_id()

                    self.active_connections.insert(peer_id);
                    self.address_lookup.insert(peer_id, endpoint.get_remote_address().clone());
                    metrics::increment_counter!("redgold.libp2p.total_established_connections");
                    metrics::increment_gauge!("redgold.libp2p.active_connections", 1.0);
                    info!(
                        "ConnectionEstablished IS_DIALER={} PEER_ID={} LOCAL_ID={} NUM_ESTABLISHED={} ACTIVE_CONNECTIONS={}",
                        endpoint.is_dialer(),
                        peer_id.short_id(),
                        self.swarm.local_peer_id().short_id(),
                        num_established,
                        self.active_connections.len()
                    );
                }
                SwarmEvent::ConnectionClosed {
                    peer_id,
                    endpoint,
                    num_established,
                    cause,
                } => {
                    self.address_lookup.remove(&peer_id);
                    metrics::decrement_gauge!("libp2p.active_connections", 1.0);
                    info!(
                        "ConnectionClosed \
                        CAUSE={:?} \
                        PEER_ID={} \
                        LOCAL_ID={} \
                        NUM_ESTABLISHED={} \
                        ACTIVE_CONNECTIONS={} \"
                        ENDPOINT={:?}",
                        cause,
                        peer_id.short_id(),
                        self.swarm.local_peer_id().short_id(),
                        num_established,
                        self.active_connections.len(),
                        endpoint
                    );
                }
                // SwarmEvent::UnreachableAddr {
                //     peer_id,
                //     attempts_remaining,
                //     error,
                //     ..
                // } => {
                //     error!("UnreachableAddr");
                //     if attempts_remaining == 0 {
                //         if let Some(sender) = self.pending_dial.remove(&peer_id) {
                //             let _ = sender.send(Err(Box::new(error)));
                //         }
                //     }
                // }
                e => {
                    error!("{:?}, {:?}", self.swarm.local_peer_id(), e)
                }
            }
        }

        async fn handle_command(&mut self, command: Command) {
            match command {
                Command::StartListening { addr, sender } => {
                    let _ = match self.swarm.listen_on(addr) {
                        Ok(_) => sender.send(Ok(())),
                        Err(e) => sender.send(Err(Box::new(e))),
                    };
                }
                Command::Dial {
                    peer_id,
                    peer_addr,
                    sender,
                } => {
                    if self.pending_dial.contains_key(&peer_id) {
                        todo!("Already dialing peer.");
                    } else {
                        info!("Attempting a dial, how to figure out if already dialed");
                        if self.active_connections.contains(&peer_id) {
                            info!("Aborting re-dial as connection already exists");
                            sender.send(Ok(())).expect("Sent");
                            return;
                        }
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, peer_addr.clone());
                        match self
                            .swarm
                            .dial(peer_addr.with(Protocol::P2p(peer_id.into())))
                        {
                            Ok(()) => {
                                // println!("pending dialed added");
                                self.pending_dial.insert(peer_id, sender);
                            }
                            Err(e) => {
                                let _ = sender.send(Err(Box::new(e)));
                            }
                        }
                    }
                }
                // Command::StartProviding { file_name, sender } => {
                //     let query_id = self
                //         .swarm
                //         .behaviour_mut()
                //         .kademlia
                //         .start_providing(file_name.into_bytes().into())
                //         .expect("No store error.");
                //     self.pending_start_providing.insert(query_id, sender);
                // }
                // Command::GetProviders { file_name, sender } => {
                //     let query_id = self
                //         .swarm
                //         .behaviour_mut()
                //         .kademlia
                //         .get_providers(file_name.into_bytes().into());
                //     self.pending_get_providers.insert(query_id, sender);
                // }
                Command::SendRequest {
                    request,
                    peer,
                    sender,
                } => {
                    info!("Sending request to peerId");
                    let request_id = self
                        .swarm
                        .behaviour_mut()
                        .request_response
                        .send_request(&peer, PeerRequest(request));
                    self.pending_requests.insert(request_id, sender);
                }
                Command::Respond {
                    response: transaction,
                    channel,
                } => {
                    // println!("Receiced command to respondTransaction");
                    self.swarm
                        .behaviour_mut()
                        .request_response
                        .send_response(channel, PeerResponse(transaction))
                        .expect("Connection to peer to be still open.");
                } // Command::DoNothing => {}
            }
        }
    }

    #[derive(NetworkBehaviour)]
    #[behaviour(event_process = false, out_event = "ComposedEvent")]
    struct ComposedBehaviour {
        request_response: RequestResponse<RequestResponseExchangeCodec>,
        kademlia: Kademlia<MemoryStore>,
    }

    #[derive(Debug)]
    enum ComposedEvent {
        RequestResponse(RequestResponseEvent<PeerRequest, PeerResponse>),
        Kademlia(KademliaEvent),
    }

    impl From<RequestResponseEvent<PeerRequest, PeerResponse>> for ComposedEvent {
        fn from(event: RequestResponseEvent<PeerRequest, PeerResponse>) -> Self {
            ComposedEvent::RequestResponse(event)
        }
    }

    impl From<KademliaEvent> for ComposedEvent {
        fn from(event: KademliaEvent) -> Self {
            ComposedEvent::Kademlia(event)
        }
    }

    #[derive(Debug)]
    pub enum Command {
        // DoNothing,
        StartListening {
            addr: Multiaddr,
            sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
        },
        Dial {
            peer_id: PeerId,
            peer_addr: Multiaddr,
            sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
        },
        // StartProviding {
        //     file_name: String,
        //     sender: oneshot::Sender<()>,
        // },
        // GetProviders {
        //     file_name: String,
        //     sender: oneshot::Sender<HashSet<PeerId>>,
        // },
        SendRequest {
            request: Request,
            peer: PeerId,
            sender: oneshot::Sender<Result<Response, Box<dyn Error + Send>>>,
        },
        Respond {
            response: Response,
            channel: ResponseChannel<PeerResponse>,
        },
    }

    pub enum Event {
        InboundRequest {
            request: Request,
            peer: PeerId,
            channel: ResponseChannel<PeerResponse>,
            remote_address: Multiaddr
        },
    }
    //
    // #[derive(Clone, Debug)]
    // pub struct InboundRequestInternal {
    //     request: Request,
    //     peer: PeerId,
    //     channel: ResponseChannelInternal,
    // }
    //
    // #[derive(Clone, Debug)]
    // pub struct ResponseChannelInternal {
    //     request_id: RequestId,
    //     peer: PeerId,
    //     a: oneshot::Sender<Response>,
    //     // CAn't do this cause it won't clone.
    //     sender_inner: oneshot::Inner<T>>,
    // }

    // Simple file exchange protocol

    #[derive(Debug, Clone)]
    struct PeerExchangeProtocol();
    #[derive(Clone)]
    struct RequestResponseExchangeCodec();
    #[derive(Debug, Clone, PartialEq)]
    struct PeerRequest(Request);
    #[derive(Debug, Clone, PartialEq)]
    pub struct PeerResponse(Response);

    impl ProtocolName for PeerExchangeProtocol {
        fn protocol_name(&self) -> &[u8] {
            "/request-response-exchange/1".as_bytes()
        }
    }

    #[async_trait]
    impl RequestResponseCodec for RequestResponseExchangeCodec {
        type Protocol = PeerExchangeProtocol;
        type Request = PeerRequest;
        type Response = PeerResponse;

        async fn read_request<T>(
            &mut self,
            _: &PeerExchangeProtocol,
            io: &mut T,
        ) -> io::Result<Self::Request>
        where
            T: AsyncRead + Unpin + Send,
        {
            let vec = read_length_prefixed(io, 1_000_000).await?;

            if vec.is_empty() {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }

            let result = Request::deserialize(vec);
            // match result {
            //     Ok(_) => {}
            //     Err(e) => {}
            // }
            Ok(PeerRequest(result?))
        }

        async fn read_response<T>(
            &mut self,
            _: &PeerExchangeProtocol,
            io: &mut T,
        ) -> io::Result<Self::Response>
        where
            T: AsyncRead + Unpin + Send,
        {
            let vec = read_length_prefixed(io, 1_000_000).await?;

            if vec.is_empty() {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }

            Ok(PeerResponse(Response::deserialize(vec)?))
        }

        async fn write_request<T>(
            &mut self,
            _: &PeerExchangeProtocol,
            io: &mut T,
            PeerRequest(data): PeerRequest,
        ) -> io::Result<()>
        where
            T: AsyncWrite + Unpin + Send,
        {
            write_length_prefixed(io, data.encode_to_vec()).await?;
            io.close().await?;

            Ok(())
        }

        async fn write_response<T>(
            &mut self,
            _: &PeerExchangeProtocol,
            io: &mut T,
            PeerResponse(data): PeerResponse,
        ) -> io::Result<()>
        where
            T: AsyncWrite + Unpin + Send,
        {
            write_length_prefixed(io, data.encode_to_vec()).await?;
            io.close().await?;

            Ok(())
        }
    }
}
