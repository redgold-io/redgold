// // Copyright 2018 Parity Technologies (UK) Ltd.
// //
// // Permission is hereby granted, free of charge, to any person obtaining a
// // copy of this software and associated documentation files (the "Software"),
// // to deal in the Software without restriction, including without limitation
// // the rights to use, copy, modify, merge, publish, distribute, sublicense,
// // and/or sell copies of the Software, and to permit persons to whom the
// // Software is furnished to do so, subject to the following conditions:
// //
// // The above copyright notice and this permission notice shall be included in
// // all copies or substantial portions of the Software.
// //
// // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// // OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// // FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// // DEALINGS IN THE SOFTWARE.
//
// //! Ping example
// //!
// //! See ../src/tutorial.rs for a step-by-step guide building the example below.
// //!
// //! In the first terminal window, run:
// //!
// //! ```sh
// //! cargo run --example ping
// //! ```
// //!
// //! It will print the PeerId and the listening addresses, e.g. `Listening on
// //! "/ip4/0.0.0.0/tcp/24915"`
// //!
// //! In the second terminal window, start a new instance of the example with:
// //!
// //! ```sh
// //! cargo run --example ping -- /ip4/127.0.0.1/tcp/24915
// //! ```
// //!
// //! The two nodes establish a connection, negotiate the ping protocol
// //! and begin pinging each other.
//
// use async_std::task::spawn;
// use async_std::task::JoinHandle;
// use async_std::{io, task};
// use bitcoin::hashes::core::task::Context;
// use crossbeam_channel::Sender;
// use future::IntoFuture;
// use futures;
// use futures::executor::block_on;
// use futures::future::PollFn;
// use futures::prelude::*;
// use libp2p::multiaddr::Multiaddr;
// use libp2p::ping::handler::PingHandler;
// use libp2p::ping::{Ping, PingConfig, PingFailure, PingSuccess};
// use libp2p::swarm::{ExpandedSwarm, Swarm, SwarmEvent};
// use libp2p::{identity, PeerId};
// use std::error::Error;
// use std::future::Pending;
// use std::str::FromStr;
// use std::task::Poll;
// use std::time::Duration;
//
// fn start_node() -> Result<(), Box<dyn Error>> {
//     let local_key = identity::Keypair::generate_ed25519();
//     let local_peer_id = PeerId::from(local_key.public());
//     println!("Local peer id: {:?}", local_peer_id);
//
//     let transport = block_on(libp2p::development_transport(local_key))?;
//
//     // Create a ping network behaviour.
//     //
//     // For illustrative purposes, the ping protocol is configured to
//     // keep the connection alive, so a continuous sequence of pings
//     // can be observed.
//     let behaviour = Ping::new(PingConfig::new().with_keep_alive(true));
//
//     let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
//
//     // Tell the swarm to listen on all interfaces and a random, OS-assigned
//     // port.
//     swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
//
//     // Dial the peer identified by the multi-address given as the second
//     // command-line argument, if any.
//     // if let Some(addr) = std::env::args().nth(1) {
//     //     let remote = addr.parse()?;
//     //     swarm.dial_addr(remote)?;
//     //     println!("Dialed {}", addr)
//     // }
//
//     let pll = future::poll_fn(move |cx| loop {
//         match swarm.poll_next_unpin(cx) {
//             Poll::Ready(Some(event)) => match event {
//                 SwarmEvent::NewListenAddr { address, .. } => {
//                     println!("Listening on {:?}", address);
//                     panic!();
//                 }
//                 SwarmEvent::Behaviour(event) => println!("{:?}", event),
//                 _ => {}
//             },
//             Poll::Ready(None) => return Poll::Ready(()),
//             Poll::Pending => return Poll::Pending,
//         }
//     });
//     block_on(pll);
//     Ok(())
// }
//
// // https://www.reddit.com/r/rust/comments/fj17z6/flume_a_100_safe_mpsc_thats_faster_than_std_and/
// // https://medium.com/@ericdreichert/how-to-print-during-rust-tests-619bdc7ccebc
// // https://stackoverflow.com/questions/57466422/how-do-i-write-an-asynchronous-function-which-polls-a-resource-and-returns-when
// #[test]
// fn testy() -> Result<(), Box<dyn Error>> {
//     println!("started");
//     //start_node()?;
//     return Ok(());
// }
