use ciborium::Value;
use std::sync::Arc;
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::Mutex,
};
use tracing::{error, info};

// 3.6 Handshake mini-protocol implementation

pub const MINI_PROTOCOL_ID_HANDSHAKE: u16 = 0;

type Index = i128;
type VersionNumber = i128;
type RefuseReasonMessage = String;
type NetworkMagic = u32;
type InitiatorAndResponderDiffusionMode = bool;
type VersionTable = Vec<(VersionNumber, Vec<NodeToNodeVersionData>)>;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Message {
    MsgProposeVersions(Vec<ProposeVersion>),
    MsgAcceptVersion(Vec<AcceptVersion>),
    MsgRefuse(RefuseReason),
}

impl Message {
    pub fn to_value(self) -> Result<Value, String> {
        match self {
            Message::MsgProposeVersions(propose_versions) => {
                let values = propose_versions.iter().map(|v| v.to_value()).collect();
                Ok(Value::Array(values))
            }
            _ => Err("Not expecting other value".to_owned()),
        }
    }

    pub fn from_value(array: Value) -> Result<Message, String> {
        info!("from_value: {:?}", array);
        let array = array.into_array().unwrap();
        let index = i128::from(array.get(0).unwrap().as_integer().unwrap());
        match index {
            1 => {
                info!("MsgAcceptVersion");
                let version_number = array.get(1).unwrap();
                let node_to_node_version_data = array.get(2).unwrap();
                let version_number_val = match AcceptVersion::from_value(1, &version_number) {
                    Ok(vn) => vn,
                    Err(error) => {
                        error!("Failed to convert {:?}: {}", version_number, error);
                        return Err(error);
                    }
                };
                let node_to_node_version_data_val =
                    match AcceptVersion::from_value(2, &node_to_node_version_data) {
                        Ok(nd) => nd,
                        Err(error) => {
                            error!(
                                "Failed to convert {:?}: {}",
                                node_to_node_version_data, error
                            );
                            return Err(error);
                        }
                    };
                Ok(Message::MsgAcceptVersion(vec![
                    version_number_val,
                    node_to_node_version_data_val,
                ]))
            }
            2 => {
                info!("MsgRefuse");
                let value = array.get(1).unwrap();

                match RefuseReason::from_value(value) {
                    Ok(refuse_reason) => Ok(Message::MsgRefuse(refuse_reason)),
                    Err(error) => {
                        error!("Failed to convert {:?}: {}", value, error);
                        return Err(error);
                    }
                }
            }
            _ => Err("Message: Do not expect any other index!".to_owned()),
        }
    }
}

#[derive(Debug)]
pub enum ProposeVersion {
    Index(Index),
    VersionTable(VersionTable),
}

impl ProposeVersion {
    pub fn create_version_table(
        supported_versions: &Vec<i64>,
        network_magic: u32,
    ) -> ProposeVersion {
        let mut version_table: VersionTable = vec![];
        for version in supported_versions {
            let version = *version as i128;
            match version_table.binary_search_by_key(&version, |(a, _b)| *a) {
                Ok(_) => {}
                Err(idx) => {
                    version_table.insert(
                        idx,
                        (
                            version,
                            vec![
                                NodeToNodeVersionData::NetworkMagic(network_magic),
                                NodeToNodeVersionData::InitiatorAndResponderDiffusionMode(false),
                            ],
                        ),
                    );
                }
            };
        }
        ProposeVersion::VersionTable(version_table)
    }

    fn to_value(&self) -> Value {
        match self {
            ProposeVersion::Index(index) => Value::from(*index),
            ProposeVersion::VersionTable(version_table) => {
                let mut values: Vec<(Value, Value)> = vec![];
                for vt in version_table {
                    let version_number = Value::from(vt.0);
                    let mut version_data: Vec<Value> = vec![];
                    for data in &vt.1 {
                        version_data.push(data.to_value());
                    }
                    values.push((version_number, Value::Array(version_data)));
                }
                Value::Map(values)
            }
        }
    }
}

#[derive(Debug)]
pub enum AcceptVersion {
    Index(Index),
    VersionNumber(VersionNumber),
    NodeToNodeVersionData(Vec<NodeToNodeVersionData>),
}

impl AcceptVersion {
    fn from_value(index: usize, value: &Value) -> Result<AcceptVersion, String> {
        match index {
            0 => {
                let index = value.as_integer().unwrap();
                return Ok(AcceptVersion::Index(i128::from(index)));
            }
            1 => {
                let version_number = value.as_integer().unwrap();
                return Ok(AcceptVersion::VersionNumber(i128::from(version_number)));
            }
            2 => {
                let array = value.as_array().unwrap();
                let mut data: Vec<NodeToNodeVersionData> = vec![];
                for dr in array {
                    let dr_val = match NodeToNodeVersionData::from_value(dr.clone()) {
                        Ok(val) => val,
                        Err(error) => {
                            error!("Error in converting value of {:?}: {}", dr, error);
                            return Err(error);
                        }
                    };
                    data.push(dr_val);
                }
                return Ok(AcceptVersion::NodeToNodeVersionData(data));
            }
            _ => Err("AcceptVersion: Do not expect any other index!".to_owned()),
        }
    }
}

#[derive(Debug)]
pub enum RefuseReason {
    RefuseReasonVersionMismatch(Vec<VersionNumber>),
    RefuseReasonHandshakeDecodeError(VersionNumber, RefuseReasonMessage),
    RefuseReasonRefused(VersionNumber, RefuseReasonMessage),
}

impl RefuseReason {
    fn from_value(value: &Value) -> Result<RefuseReason, String> {
        let value = value.as_array().unwrap();
        let index = i128::from(value.get(0).unwrap().as_integer().unwrap());

        match index {
            0 => {
                info!("Encountered RefuseReasonVersionMismatch");
                let array = value.get(1).unwrap().as_array();
                if array.is_none() {
                    return Ok(RefuseReason::RefuseReasonVersionMismatch(vec![]));
                }
                let mut version_numbers: Vec<VersionNumber> = vec![];

                for version in array.unwrap() {
                    version_numbers.push(i128::from(version.as_integer().unwrap()));
                }
                return Ok(RefuseReason::RefuseReasonVersionMismatch(version_numbers));
            }
            1 => {
                info!("Encountered RefuseReasonHandshakeDecodeError");
                let version_number = value.get(1).unwrap().as_integer().unwrap();
                let tstr = value.get(2).unwrap().as_text().unwrap();
                return Ok(RefuseReason::RefuseReasonHandshakeDecodeError(
                    i128::from(version_number),
                    tstr.to_owned(),
                ));
            }
            2 => {
                info!("Encountered RefuseReasonRefused");
                let version_number = value.get(1).unwrap().as_integer().unwrap();
                let tstr = value.get(2).unwrap().as_text().unwrap();
                return Ok(RefuseReason::RefuseReasonRefused(
                    i128::from(version_number),
                    tstr.to_owned(),
                ));
            }
            _ => Err("RefuseReason: Do not expect any other index!".to_owned()),
        }
    }
}

#[derive(Debug)]
pub enum NodeToNodeVersionData {
    NetworkMagic(NetworkMagic),
    InitiatorAndResponderDiffusionMode(InitiatorAndResponderDiffusionMode),
}

impl NodeToNodeVersionData {
    fn to_value(&self) -> Value {
        match self {
            NodeToNodeVersionData::NetworkMagic(network_magic) => {
                Value::from(*network_magic as i128)
            }
            NodeToNodeVersionData::InitiatorAndResponderDiffusionMode(
                initiator_and_responder_diffusion_mode,
            ) => Value::Bool(*initiator_and_responder_diffusion_mode),
        }
    }

    fn from_value(value: Value) -> Result<NodeToNodeVersionData, String> {
        if value.is_integer() {
            let network_magic: i128 = value.as_integer().unwrap().into();
            info!("value: {:?}", network_magic);
            return Ok(NodeToNodeVersionData::NetworkMagic(network_magic as u32));
        } else if value.is_bool() {
            return Ok(NodeToNodeVersionData::InitiatorAndResponderDiffusionMode(
                value.as_bool().unwrap(),
            ));
        } else {
            return Err("Do not expect any other value!".to_owned());
        }
    }
}

/*
#[derive(Debug)]
enum Agency {
    Client,
    Server,
}
*/

// Handshake Mini-Protocol state machine goes through following states
#[derive(Debug)]
#[allow(dead_code)]
pub enum StateMachine {
    StPropose,
    StConfirm,
    StDone,
}

pub struct NodeConfig<'a> {
    pub host: &'a str,
    pub magic: u32,
    pub network_id: &'a str,
    pub read: Arc<Mutex<Box<dyn AsyncRead + Send + Unpin>>>,
    pub write: Mutex<Box<dyn AsyncWrite + Send + Unpin>>,
}

impl<'a> NodeConfig<'a> {
    pub async fn init(
        host: &'a str,
        magic: u32,
        network_id: &'a str,
    ) -> Result<NodeConfig<'a>, String> {
        info!("Connecting host: {:?}", host);
        let stream = match TcpStream::connect(host).await {
            Ok(stream) => stream,
            Err(error) => {
                error!("Failed to connect: {}", error);
                return Err(error.to_string());
            }
        };
        info!("Stream created for {}", network_id);
        let (read, write) = io::split(stream);
        Ok(NodeConfig {
            host,
            magic,
            network_id,
            read: Arc::new(Mutex::new(Box::new(read))),
            write: Mutex::new(Box::new(write)),
        })
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn it_works() {}
}
