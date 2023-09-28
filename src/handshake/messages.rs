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

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum Message {
    // MsgProposeVersions
    ProposeVersions(Vec<ProposeVersion>),
    // MsgAcceptVersion
    AcceptVersion(Vec<AcceptVersion>),
    // MsgRefuse
    Refuse(RefuseReason),
}

impl Message {
    pub fn to_value(&self) -> Result<Value, String> {
        match self {
            Message::ProposeVersions(propose_versions) => {
                let values = propose_versions.iter().map(|v| v.to_value()).collect();
                Ok(Value::Array(values))
            }
            _ => Err("Not expecting other value".to_owned()),
        }
    }

    pub fn from_value(array: Value) -> Result<Message, String> {
        let array = array
            .clone()
            .into_array()
            .map_err(|error| format!("Could not convert Message into array: {:?}", error))?;

        let index = array.get(0).ok_or("No value found at message index 0")?;
        let index = index
            .as_integer()
            .ok_or("Could not convert index to integer")?;
        let index = i128::from(index);
        match index {
            1 => {
                info!("AcceptVersion");
                let version_number = array
                    .get(1)
                    .ok_or("No value found at AcceptVersion index 1")?;
                let node_to_node_version_data = array
                    .get(2)
                    .ok_or("No value found at AcceptVersion index 2")?;

                let version_number_val =
                    match AcceptVersion::version_number_from_value(version_number) {
                        Ok(vn) => vn,
                        Err(error) => {
                            error!("Failed to convert {:?}: {}", version_number, error);
                            return Err(error);
                        }
                    };
                let node_to_node_version_data_val =
                    match AcceptVersion::nodes_data_from_value(node_to_node_version_data) {
                        Ok(nd) => nd,
                        Err(error) => {
                            error!(
                                "Failed to convert {:?}: {}",
                                node_to_node_version_data, error
                            );
                            return Err(error);
                        }
                    };
                Ok(Message::AcceptVersion(vec![
                    AcceptVersion::Index(1),
                    version_number_val,
                    node_to_node_version_data_val,
                ]))
            }
            2 => {
                info!("Refuse");
                let value = array.get(1).ok_or("No value found at Refuse index 1")?;

                match RefuseReason::from_value(value) {
                    Ok(refuse_reason) => Ok(Message::Refuse(refuse_reason)),
                    Err(error) => {
                        error!("Failed to convert {:?}: {}", value, error);
                        Err(error)
                    }
                }
            }
            _ => Err(format!("Message: Do not expect any other index {}!", index)),
        }
    }
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub enum AcceptVersion {
    Index(Index),
    VersionNumber(VersionNumber),
    NodeToNodeVersionData(Vec<NodeToNodeVersionData>),
}

impl AcceptVersion {
    fn version_number_from_value(value: &Value) -> Result<AcceptVersion, String> {
        let version_number = value
            .as_integer()
            .ok_or("Could not convert version_number as integer")?;
        Ok(AcceptVersion::VersionNumber(VersionNumber::from(
            version_number,
        )))
    }

    fn nodes_data_from_value(value: &Value) -> Result<AcceptVersion, String> {
        let array = value.as_array().ok_or("Could not convert to array")?;
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
        Ok(AcceptVersion::NodeToNodeVersionData(data))
    }
}

#[derive(Debug, PartialEq)]
pub enum RefuseReason {
    // RefuseReasonVersionMismatch
    VersionMismatch(Vec<VersionNumber>),
    // RefuseReasonHandshakeDecodeError
    HandshakeDecodeError(VersionNumber, RefuseReasonMessage),
    // RefuseReasonRefused
    Refused(VersionNumber, RefuseReasonMessage),
}

impl RefuseReason {
    fn from_value(value: &Value) -> Result<RefuseReason, String> {
        let value = value.as_array().ok_or("No value found!")?;

        let index = value
            .get(0)
            .ok_or("No value found at RefuseReason index 0")?;
        let index = index
            .as_integer()
            .ok_or("Could not convert index to integer")?;
        let index = i128::from(index);
        match index {
            0 => {
                info!("Encountered VersionMismatch");
                let array = value
                    .get(1)
                    .ok_or("No value found at VersionMismatch index 1")?
                    .as_array();

                if array.is_none() {
                    return Ok(RefuseReason::VersionMismatch(vec![]));
                }
                let mut version_numbers: Vec<VersionNumber> = vec![];

                for version in array.unwrap() {
                    version_numbers.push(i128::from(
                        version
                            .as_integer()
                            .ok_or("Could not convert version_number to integer")?,
                    ));
                }
                Ok(RefuseReason::VersionMismatch(version_numbers))
            }
            1 => {
                info!("Encountered HandshakeDecodeError");
                let version_number = value
                    .get(1)
                    .ok_or("No value found at HandshakeDecodeError index 1")?;
                let version_number = version_number
                    .as_integer()
                    .ok_or("Could not convert version_number to integer")?;

                let tstr = value
                    .get(2)
                    .ok_or("No value found at HandshakeDecodeError index 2")?;
                let tstr = tstr.as_text().ok_or("Could not convert tstr to text")?;

                Ok(RefuseReason::HandshakeDecodeError(
                    i128::from(version_number),
                    tstr.to_owned(),
                ))
            }
            2 => {
                info!("Encountered Refused");
                let version_number = value.get(1).ok_or("No value found at Refused index 1")?;
                let version_number = version_number
                    .as_integer()
                    .ok_or("Could not convert to integer")?;

                let tstr = value.get(2).ok_or("No value found at Refused index 2")?;
                let tstr = tstr.as_text().ok_or("Could not convert to integer")?;

                Ok(RefuseReason::Refused(
                    i128::from(version_number),
                    tstr.to_owned(),
                ))
            }
            _ => Err("RefuseReason: Do not expect any other index!".to_owned()),
        }
    }
}

#[derive(Debug, PartialEq)]
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
            let network_magic: i128 = value
                .as_integer()
                .ok_or("Could not convert network_magic to integer.")?
                .into();
            info!("value: {:?}", network_magic);
            Ok(NodeToNodeVersionData::NetworkMagic(network_magic as u32))
        } else if value.is_bool() {
            Ok(NodeToNodeVersionData::InitiatorAndResponderDiffusionMode(
                value.as_bool().ok_or("Could not convert mode to bool")?,
            ))
        } else {
            Err("Do not expect any other value!".to_owned())
        }
    }
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum Agency {
    Client,
    Server,
}

// Handshake Mini-Protocol state machine goes through following states
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum StateMachine {
    // StPropose
    Propose,
    // StConfirm
    Confirm,
    // StDone
    Done,
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
    use super::*;

    #[tokio::test]
    async fn propose_versions() {
        let message = Message::ProposeVersions(vec![
            ProposeVersion::Index(0),
            ProposeVersion::create_version_table(&vec![7, 8, 9, 10], 1),
        ]);
        assert!(format!("{:?}", message).eq("ProposeVersions([Index(0), VersionTable([(7, [NetworkMagic(1), InitiatorAndResponderDiffusionMode(false)]), (8, [NetworkMagic(1), InitiatorAndResponderDiffusionMode(false)]), (9, [NetworkMagic(1), InitiatorAndResponderDiffusionMode(false)]), (10, [NetworkMagic(1), InitiatorAndResponderDiffusionMode(false)])])])"));

        let value = message.to_value().unwrap();
        assert!(format!("{:?}", value).eq("Array([Integer(Integer(0)), Map([(Integer(Integer(7)), Array([Integer(Integer(1)), Bool(false)])), (Integer(Integer(8)), Array([Integer(Integer(1)), Bool(false)])), (Integer(Integer(9)), Array([Integer(Integer(1)), Bool(false)])), (Integer(Integer(10)), Array([Integer(Integer(1)), Bool(false)]))])])"));
    }

    #[tokio::test]
    async fn accept_versions() {
        let value = Value::Array(vec![
            Value::from(1),
            Value::from(10),
            Value::Array(vec![Value::from(1), Value::Bool(false)]),
        ]);
        assert!(format!("{:?}", value).eq("Array([Integer(Integer(1)), Integer(Integer(10)), Array([Integer(Integer(1)), Bool(false)])])"));

        let message = Message::from_value(value).unwrap();
        assert!(format!("{:?}", message).eq("AcceptVersion([Index(1), VersionNumber(10), NodeToNodeVersionData([NetworkMagic(1), InitiatorAndResponderDiffusionMode(false)])])"));
    }

    #[tokio::test]
    async fn refuse_reason_version_mismatch() {
        let value = Value::Array(vec![
            Value::from(2),
            Value::Array(vec![
                Value::from(0),
                Value::Array(vec![
                    Value::from(7),
                    Value::from(8),
                    Value::from(9),
                    Value::from(10),
                ]),
            ]),
        ]);
        assert!(format!("{:?}", value).eq("Array([Integer(Integer(2)), Array([Integer(Integer(0)), Array([Integer(Integer(7)), Integer(Integer(8)), Integer(Integer(9)), Integer(Integer(10))])])])"));

        let message = Message::from_value(value).unwrap();
        assert!(format!("{:?}", message).eq("Refuse(VersionMismatch([7, 8, 9, 10]))"));
    }

    #[tokio::test]
    async fn refuse_reason_handshake_decode_error() {
        let value = Value::Array(vec![
            Value::from(2),
            Value::Array(vec![
                Value::from(1),
                Value::from(11),
                Value::Text("unknown encoding: TList [TInt 1,TBool False]".to_owned()),
            ]),
        ]);
        assert!(format!("{:?}", value).eq("Array([Integer(Integer(2)), Array([Integer(Integer(1)), Integer(Integer(11)), Text(\"unknown encoding: TList [TInt 1,TBool False]\")])])"));

        let message = Message::from_value(value).unwrap();
        assert!(format!("{:?}", message).eq(
            "Refuse(HandshakeDecodeError(11, \"unknown encoding: TList [TInt 1,TBool False]\"))"
        ));
    }

    #[tokio::test]
    async fn refuse_reason_refused() {
        let value = Value::Array(vec![
            Value::from(2),
            Value::Array(vec![
                Value::from(2),
                Value::from(10),
                Value::Text("unknown reason".to_owned()),
            ]),
        ]);
        assert!(format!("{:?}", value).eq("Array([Integer(Integer(2)), Array([Integer(Integer(2)), Integer(Integer(10)), Text(\"unknown reason\")])])"));

        let message = Message::from_value(value).unwrap();
        println!("{:?}", message);
        assert!(format!("{:?}", message).eq("Refuse(Refused(10, \"unknown reason\"))"));
    }
}
