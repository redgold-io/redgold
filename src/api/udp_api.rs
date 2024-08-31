#![cfg(not(target_os = "wasi"))] // Wasi doesn't support UDP

use std::collections::HashMap;
use std::future::Future;
use tokio::net::UdpSocket;
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, Encoder, LinesCodec};
use tokio_util::udp::UdpFramed;

use bytes::{BufMut, BytesMut};
use futures::future::try_join;
use futures::future::FutureExt;
use futures::sink::SinkExt;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use futures::{Stream, TryStreamExt};
use futures::stream::{BoxStream, SplitSink, SplitStream};
use itertools::Itertools;
use tokio_stream::wrappers::IntervalStream;
use tokio_util::either::Either;
use uuid::Uuid;
use redgold_keys::request_support::RequestSupport;
use redgold_schema::{bytes_data, ErrorInfoContext};
use redgold_schema::structs::{ErrorInfo, Request, UdpMessage};
use crate::core::internal_message::{Channel, new_channel, PeerMessage, RecvAsyncErrorInfo, SendErrorInfo};
use crate::core::relay::Relay;
use crate::util;
use crate::util::keys::public_key_from_bytes;
use crate::util::random_port;
use redgold_schema::proto_serde::ProtoSerde;
use crate::api::udp_api::UdpOperation::Outgoing;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::easy_json::json;
use redgold_schema::proto_serde::ProtoHashable;

#[cfg_attr(any(target_os = "macos", target_os = "ios"), allow(unused_assignments))]
#[tokio::test]
async fn send_framed_byte_codec() -> std::io::Result<()> {
    let mut a_soc = UdpSocket::bind("127.0.0.1:0").await?;
    let mut b_soc = UdpSocket::bind("127.0.0.1:0").await?;

    let a_addr = a_soc.local_addr()?;
    let b_addr = b_soc.local_addr()?;

    // test sending & receiving bytes
    {
        let mut a = UdpFramed::new(a_soc, ByteCodec);
        let mut b = UdpFramed::new(b_soc, ByteCodec);

        let msg = b"4567";

        let send = a.send((msg, b_addr));
        let recv = b.next().map(|e| e.unwrap());
        let (_, received) = try_join(send, recv).await.unwrap();

        let (data, addr) = received;
        assert_eq!(msg, &*data);
        assert_eq!(a_addr, addr);

        a_soc = a.into_inner();
        b_soc = b.into_inner();
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    // test sending & receiving an empty message
    {
        let mut a = UdpFramed::new(a_soc, ByteCodec);
        let mut b = UdpFramed::new(b_soc, ByteCodec);

        let msg = b"";

        let send = a.send((msg, b_addr));
        let recv = b.next().map(|e| e.unwrap());
        let (_, received) = try_join(send, recv).await.unwrap();

        let (data, addr) = received;
        assert_eq!(msg, &*data);
        assert_eq!(a_addr, addr);
    }

    Ok(())
}

pub struct ByteCodec;

impl Decoder for ByteCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Vec<u8>>, io::Error> {
        let len = buf.len();
        Ok(Some(buf.split_to(len).to_vec()))
    }
}

impl Encoder<&[u8]> for ByteCodec {
    type Error = io::Error;

    fn encode(&mut self, data: &[u8], buf: &mut BytesMut) -> Result<(), io::Error> {
        buf.reserve(data.len());
        buf.put_slice(data);
        Ok(())
    }
}

#[tokio::test]
async fn send_framed_lines_codec() -> std::io::Result<()> {
    let a_soc = UdpSocket::bind("127.0.0.1:0").await?;
    let b_soc = UdpSocket::bind("127.0.0.1:0").await?;

    let a_addr = a_soc.local_addr()?;
    let b_addr = b_soc.local_addr()?;

    let mut a = UdpFramed::new(a_soc, ByteCodec);
    let mut b = UdpFramed::new(b_soc, LinesCodec::new());

    let msg = b"1\r\n2\r\n3\r\n".to_vec();
    a.send((&msg, b_addr)).await?;

    assert_eq!(b.next().await.unwrap().unwrap(), ("1".to_string(), a_addr));
    assert_eq!(b.next().await.unwrap().unwrap(), ("2".to_string(), a_addr));
    assert_eq!(b.next().await.unwrap().unwrap(), ("3".to_string(), a_addr));

    Ok(())
}

#[tokio::test]
async fn framed_half() -> std::io::Result<()> {
    let a_soc = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);
    let b_soc = a_soc.clone();

    let a_addr = a_soc.local_addr()?;
    let b_addr = b_soc.local_addr()?;

    let mut a = UdpFramed::new(a_soc, ByteCodec);
    let mut b = UdpFramed::new(b_soc, LinesCodec::new());

    let msg = b"1\r\n2\r\n3\r\n".to_vec();
    a.send((&msg, b_addr)).await?;

    let msg = b"4\r\n5\r\n6\r\n".to_vec();
    a.send((&msg, b_addr)).await?;

    let option = b.next().await;
    let x = option.unwrap().unwrap();
    assert_eq!(x, ("1".to_string(), a_addr));
    assert_eq!(b.next().await.unwrap().unwrap(), ("2".to_string(), a_addr));
    assert_eq!(b.next().await.unwrap().unwrap(), ("3".to_string(), a_addr));

    assert_eq!(b.next().await.unwrap().unwrap(), ("4".to_string(), a_addr));
    assert_eq!(b.next().await.unwrap().unwrap(), ("5".to_string(), a_addr));
    assert_eq!(b.next().await.unwrap().unwrap(), ("6".to_string(), a_addr));

    Ok(())
}
//
// struct UdpMessageWrapper {
//     udp_message: UdpMessage,
// }

struct UdpMessageCodec {}

impl Decoder for UdpMessageCodec {
    type Item = UdpMessage;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<UdpMessage>, io::Error> {
        let len = buf.len();
        if len == 0 {
            return Ok(None);
        }
        let vec = buf.split_to(len).to_vec();
        // Wow this call succeeds on empty vec???
        let udp_deser = UdpMessage::proto_deserialize(vec);
        let option = udp_deser.ok();
        Ok(option)
    }
}

impl Encoder<UdpMessage> for UdpMessageCodec {
    type Error = io::Error;

    fn encode(&mut self, data_msg: UdpMessage, buf: &mut BytesMut) -> Result<(), io::Error> {
        let data = data_msg.proto_serialize();
        buf.reserve(data.len());
        buf.put_slice(&*data);
        Ok(())
    }
}



pub trait UdpMessageSupport {
    fn new(data: Vec<u8>, part: i64, parts: i64) -> Self;
}
impl UdpMessageSupport for UdpMessage {
    fn new(data: Vec<u8>, part: i64, parts: i64) -> Self {
        UdpMessage {
            bytes: bytes_data(data),
            part,
            parts,
            uuid: Uuid::new_v4().to_string(),
            timestamp: util::current_time_millis_i64(),
        }
    }
}

struct UdpServer {
    // socket: UdpSocket,
    relay: Relay,
    // TODO: optimize reassembly with parts array?
    messages: HashMap<String, (Vec<UdpMessage>, SocketAddr)>,
    sink: SplitSink<UdpFramed<UdpMessageCodec>, (UdpMessage, SocketAddr)>,
}



const UDP_CHUNK_SIZE : usize = 1024;

impl UdpServer {
    async fn new(relay: Relay, port: Option<u16>) -> Result<(), ErrorInfo> {
        let port = port.unwrap_or(0);
        let addr = format!("0.0.0.0:{}", port.to_string());
        let socket =
            UdpSocket::bind(addr)
                .await
                .error_info("Failed to bind UDP socket")?;
        let framed = UdpFramed::new(socket, UdpMessageCodec{});
        let (sink, stream) = futures::StreamExt::split(framed);
        let mut server = Self {
            // socket,
            sink,
            relay,
            messages: Default::default(),
        };
        server.run(stream).await
    }

    // TODO: metrics for bad messages
    async fn run(&mut self, stream: SplitStream<UdpFramed<UdpMessageCodec>>) -> Result<(), ErrorInfo> {

        // Change to stream

        let interval = tokio::time::interval(std::time::Duration::from_secs(100));
        let interval_stream = IntervalStream::new(interval).map(|_| UdpOperation::Interval);
        let incoming_stream = stream
            .map(|m| UdpOperation::Incoming(m.error_info("Failed to receive UDP message")));
        let r = self.relay.udp_outgoing_messages.receiver.clone();
        let outgoing_stream = r.into_stream().map(|m| UdpOperation::Outgoing(m));

        let interval_stream = futures::StreamExt::boxed(interval_stream);
        let incoming_stream = futures::StreamExt::boxed(incoming_stream);
        let outgoing_stream = futures::StreamExt::boxed(outgoing_stream);

        futures::stream::select_all(vec![incoming_stream, interval_stream, outgoing_stream])
            .map(|x| {
                let res: Result<UdpOperation, ErrorInfo> = Ok(x);
                res
            })
            .try_fold(self, |server, o| async move {
                server.handle_udp_operation(o).await.and_then(|_| Ok(server))
            })
            .await?;
        Ok(())
    }

    async fn send_rx_incoming_log(&mut self, data: Vec<u8>, addr: SocketAddr) -> Result<(), ErrorInfo> {
        self.send_rx_incoming(data, addr.clone()).await.map_err(|e| {
            log::error!("Failed to send UDP message to relay: {}", e.json_or());
            e
        })
}

    async fn send_rx_incoming(&mut self, data: Vec<u8>, addr: SocketAddr) -> Result<(), ErrorInfo> {
        let req = Request::proto_deserialize(data)?;
        let node_pk = req.verify_auth()?;
        let mut pm = PeerMessage::empty();
        pm.public_key = Some(node_pk.clone());
        pm.socket_addr = Some(addr);
        pm.request = req;
        self.relay.peer_message_rx.sender.send_rg_err(pm)?;
        Ok(())
    }

    async fn process_typed(&mut self, typed: Result<(UdpMessage, SocketAddr), ErrorInfo>) -> Result<(), ErrorInfo> {
        match typed {
            Err(e) => {
                log::error!("UDP error: {}", e.json_or());
            }
            Ok((wrapper, addr)) => {
                let w = wrapper.clone();
                let json_msg = json(&w).expect("json");
                log::info!("UDP message received from: {} - contents - {}", addr.clone(), json_msg);

                let mut message = wrapper.clone();
                message.timestamp = util::current_time_millis() as i64;
                if message.parts == 1 {
                    if let Some(data) = message.bytes.map(|b| b.value) {
                        self.send_rx_incoming_log(data, addr.clone()).await.ok();
                    }
                } else {
                    match self.messages.get_mut(&message.uuid.clone()) {
                        Some((parts, _stored_addr)) => {
                            parts.push(message.clone());
                            if parts.len() == (message.parts as usize) {
                                let mut data: Vec<u8> = Vec::new();
                                parts.sort_by(|a, b| a.part.cmp(&b.part));
                                for part in parts {
                                    if let Some(b) = &part.bytes {
                                        data.extend_from_slice(&*b.value);
                                    }
                                }
                                // Message is complete, send it to the relay
                                self.send_rx_incoming_log(data, addr.clone()).await.ok();
                                self.messages.remove(&wrapper.uuid.clone());
                            }
                        },
                        None => {
                            let mut parts = Vec::new();
                            parts.push(message.clone());
                            self.messages.insert(message.uuid, (parts, addr));
                        }
                    }
                }
            },
        }
        Ok(())
    }

    async fn handle_udp_operation(&mut self, udp_operation: UdpOperation) -> Result<(), ErrorInfo> {
        match udp_operation {
            Outgoing(pm) => {
                if let Some(b_addr) = pm.socket_addr {
                    let ser = pm.request.proto_serialize();
                    let chunks = ser.chunks(UDP_CHUNK_SIZE);
                    let parts = chunks.len();
                    for (i, chunk) in chunks.enumerate() {
                        let msg = UdpMessage::new(chunk.to_vec(), i as i64, parts as i64);
                        log::debug!("Sending UDP message to {}", b_addr);
                        self.sink.send((msg, b_addr)).await.error_info("Failed to send UDP message")?;
                    }
                }
                Ok(())
            },
            UdpOperation::Incoming(i) => {
                self.process_typed(i).await?;
                Ok(())
            }
            UdpOperation::Interval => {
                let mut stale_messages = vec![];
                for (i, (m, _)) in &mut self.messages.iter() {
                    let stale = m.iter()
                        .find(|m| ((m.timestamp + 1000*100) as u64) < util::current_time_millis())
                        .is_some();
                    if stale {
                        // self.messages.remove(i);
                        stale_messages.push(i.clone());
                    }
                }
                for i in stale_messages {
                    self.messages.remove(&i);
                }
                Ok(())
            }
        }
    }

}

enum UdpOperation {
    Outgoing(PeerMessage),
    Incoming(Result<(UdpMessage, SocketAddr), ErrorInfo>),
    Interval,
}

#[ignore]
#[tokio::test]
async fn send_request_internal() -> std::io::Result<()> {
    let a_soc = UdpSocket::bind("127.0.0.1:0").await?;
    let b_soc = UdpSocket::bind("127.0.0.1:0").await?;

    // let a_addr = a_soc.local_addr()?;
    let b_addr = b_soc.local_addr()?;

    let mut a = UdpFramed::new(a_soc, UdpMessageCodec{});
    let mut b = UdpFramed::new(b_soc, UdpMessageCodec{});

    let msg = Request::empty().about().proto_serialize();
    let msg = UdpMessage::new(msg, 0, 1);
    a.send((msg.clone(), b_addr)).await?;

    let option = b.next().await;
    let x = option.unwrap().unwrap().0;
    let dbg = x.clone();
    println!("option: {}", json(&dbg).expect(""));
    assert_eq!(x, msg);

    Ok(())
}
// did these break CI?
#[ignore]
#[tokio::test]
async fn servers_multiple() -> std::io::Result<()> {

    // util::init_logger();
    let port1 = random_port();
    let port2 = random_port();
    println!("port 1: {}, port 2: {}", port1.to_string(), port2.to_string());
    let relay1 = Relay::default().await;
    let relay2 = Relay::default().await;
    tokio::spawn(UdpServer::new(relay1.clone(), Some(port1)));
    tokio::spawn(UdpServer::new(relay2.clone(), Some(port2)));

    // let socket_addr1 = SocketAddr::new(IpAddr::from_str("127.0.0.1").expect(""), port1);
    let socket_addr2 = SocketAddr::new(IpAddr::from_str("127.0.0.1").expect(""), port2);

    let mut pm = PeerMessage::empty();
    let pair = relay1.node_config.words().default_kp().expect("").clone();
    let request = Request::empty().about();
    let msg = request.with_auth(&pair);
    msg.verify_auth().expect("");
    pm.request = msg.clone();
    pm.socket_addr = Some(socket_addr2);

    relay1.udp_outgoing_messages.sender.send(pm.clone()).expect("");
    let output = relay2.peer_message_rx.receiver.recv_async_err().await.expect("");
    assert_eq!(pm.request.proto_serialize(), output.request.proto_serialize());
    Ok(())
}
