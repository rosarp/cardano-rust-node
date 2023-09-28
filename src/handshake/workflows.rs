use super::{Message, NodeConfig, ProposeVersion, StateMachine, MINI_PROTOCOL_ID_HANDSHAKE};
use ciborium::Value;
use ciborium::{from_reader, into_writer};
use core::panic;
use std::sync::Arc;
use std::vec;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    join,
    sync::Mutex,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, warn};

pub async fn negotiate<'a>(
    node_config: NodeConfig<'a>,
    supported_versions: &Vec<i64>,
) -> Result<(Duration, Duration), String> {
    let start = Instant::now();

    let mut message = Vec::new();
    prepare_message(
        &mut message,
        StateMachine::Propose,
        supported_versions,
        node_config.magic,
        node_config.network_id,
    );

    let negotiate_start = Instant::now();
    join!(
        receive(node_config.read, node_config.network_id),
        send(node_config.write, message, node_config.network_id)
    );

    Ok((negotiate_start.elapsed(), start.elapsed()))
}

fn prepare_message(
    message: &mut Vec<u8>,
    state: StateMachine,
    supported_versions: &Vec<i64>,
    network_magic: u32,
    network_id: &str,
) {
    match state {
        StateMachine::Propose => {
            let propose_versions = Message::ProposeVersions(vec![
                ProposeVersion::Index(0),
                ProposeVersion::create_version_table(supported_versions, network_magic),
            ]);
            info!("Sending {} : {:?}", network_id, propose_versions);
            let propose_versions = match propose_versions.to_value() {
                Ok(pv) => pv,
                Err(error) => panic!("Failed to create message: {}", error),
            };
            debug!("propose_versions {}: {:?}", network_id, propose_versions);
            into_writer(&propose_versions, message).unwrap();
        }
        StateMachine::Confirm => {}
        StateMachine::Done => panic!("Not expecting Done status!"),
    }
}

async fn send(
    write: Mutex<Box<dyn AsyncWrite + Send + Unpin>>,
    message: Vec<u8>,
    network_id: &str,
) {
    info!("Sending MsgProposeVersions: {}", network_id);
    let mut write = write.lock().await;
    let transmission_time = Instant::now().elapsed().as_micros() as u32;
    let capacity = 32 + 16 + 16 + message.len();
    let mut write_buffer: Vec<u8> = Vec::with_capacity(capacity);
    let transmission_time = transmission_time.to_be_bytes();
    for v in transmission_time {
        write_buffer.push(v);
    }
    let mode = MINI_PROTOCOL_ID_HANDSHAKE.to_be_bytes();
    mode.into_iter().for_each(|m| write_buffer.push(m));
    let message_len = (message.len() as u16).to_be_bytes();
    message_len.into_iter().for_each(|m| write_buffer.push(m));
    message.into_iter().for_each(|m| write_buffer.push(m));
    match write.write_all(&write_buffer).await {
        Ok(_) => info!("Successfully sent request to {}", network_id),
        Err(error) => error!("Error sending request to server: {:?}", error),
    }
}

async fn receive(read: Arc<Mutex<Box<dyn AsyncRead + Send + Unpin>>>, network_id: &str) {
    info!("Reading response: {}", network_id);
    let mut read = read.lock().await;

    let mut response_received: Vec<u8> = Vec::new();
    match read.read_buf(&mut response_received).await {
        Ok(count) => info!("Bytes read: {}", count),
        Err(error) => {
            warn!("Network Id: {}, Error: {:?}", network_id, error);
            error!("Error in reading response from server");
            return;
        }
    };
    info!(
        "Read success {} Response Received: {:?}",
        network_id, response_received
    );

    match response_received[0..4].try_into() {
        Ok(bytes) => {
            let val = u32::from_be_bytes(bytes);
            info!("transmission_time: {}", val);
        }
        Err(error) => error!(
            "Error reading transmission_time for {}: {}",
            network_id, error
        ),
    }

    match response_received[4..6].try_into() {
        Ok(bytes) => {
            let val = u16::from_be_bytes(bytes);
            info!("protocol_id: {}", val);
        }
        Err(error) => error!("Error reading protocol_id for {}: {}", network_id, error),
    }

    match response_received[6..8].try_into() {
        Ok(bytes) => {
            let val = u16::from_be_bytes(bytes);
            info!("message_len: {}", val);
        }
        Err(error) => error!("Error reading message_len for {}: {}", network_id, error),
    };

    let response_message: Value = from_reader(&response_received[8..]).unwrap();
    debug!("response_message {}: {:?}", network_id, response_message);
    match Message::from_value(response_message) {
        Ok(response_message) => info!("response_message {}: {:?}", network_id, response_message),
        Err(error) => error!("Error message: {}", error),
    };

    info!("Reading Complete: {}", network_id);
}
