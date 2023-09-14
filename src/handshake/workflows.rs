use super::{Message, NodeConfig, ProposeVersion, StateMachine, MINI_PROTOCOL_ID_HANDSHAKE};
use ciborium::Value;
use ciborium::{from_reader, into_writer};
use core::panic;
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    join,
    sync::Mutex,
    time::{sleep, Duration, Instant},
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
        StateMachine::StPropose,
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
        StateMachine::StPropose => {
            let propose_versions = Message::MsgProposeVersions(vec![
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
        StateMachine::StConfirm => {}
        StateMachine::StDone => panic!("Not expecting Done status!"),
    }
}

async fn send(
    write: Mutex<Box<dyn AsyncWrite + Send + Unpin>>,
    message: Vec<u8>,
    network_id: &str,
) {
    info!("Sending MsgProposeVersions: {}", network_id);
    let mode = 0;
    let mode_and_mini_protocol_id = if mode == 1 {
        128 + MINI_PROTOCOL_ID_HANDSHAKE
    } else {
        MINI_PROTOCOL_ID_HANDSHAKE
    };
    let mut write = write.lock().await;
    let transmission_time = Instant::now().elapsed().as_micros() as u32;
    write.write_u32(transmission_time).await.unwrap();
    write.write_u16(mode_and_mini_protocol_id).await.unwrap();
    write.write_u16(message.len() as u16).await.unwrap();
    write.write_all(&message).await.unwrap();

    info!("Send Comlete: {}", network_id);
}

async fn receive(read: Arc<Mutex<Box<dyn AsyncRead + Send + Unpin>>>, network_id: &str) {
    info!("Reading response: {}", network_id);
    let mut read = read.lock().await;
    let transmission_time = match read.read_u32().await {
        Ok(time) => time,
        Err(error) => {
            error!("Server returned error: {}", error);
            return;
        }
    };
    info!("transmission_time {}: {:?}", network_id, transmission_time);
    let protocol_id = read.read_u16().await.unwrap();
    info!("protocol_id {}: {}", network_id, protocol_id);
    let message_len = read.read_u16().await.unwrap();

    let mut response_message = vec![0u8; message_len as usize];
    match read.read_exact(&mut response_message).await {
        Ok(result) => info!("Read success {}: {} bytes", network_id, result),
        Err(error) => {
            warn!("Network: {}, Error: {:?}", network_id, error);
            sleep(Duration::from_secs(5)).await;
        }
    }

    let response_message: Value = from_reader(&response_message[..]).unwrap();
    debug!("response_message {}: {:?}", network_id, response_message);
    match Message::from_value(response_message) {
        Ok(response_message) => info!("response_message {}: {:?}", network_id, response_message),
        Err(error) => error!("Error message: {}", error),
    };

    info!("Reading Complete: {}", network_id);
}
