use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

pub const ORACLE_ANALYSIS_SERVICE_PROTOCOL: &str = "oracle-analysis-jsonl";
pub const ORACLE_ANALYSIS_SERVICE_PROTOCOL_VERSION: u32 = 1;
pub const ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA: &str = "OracleAnalysisServiceEndpoint";
pub const ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct OracleAnalysisServiceRequestV1 {
    pub id: Option<Value>,
    pub auth_token: Option<String>,
    pub command: OracleAnalysisServiceCommandV1,
}

impl<'de> Deserialize<'de> for OracleAnalysisServiceRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut object = serde_json::Map::<String, Value>::deserialize(deserializer)?;
        let id = object.remove("id");
        let auth_token = object
            .remove("auth_token")
            .map(serde_json::from_value::<String>)
            .transpose()
            .map_err(D::Error::custom)?;
        let command = OracleAnalysisServiceCommandV1::deserialize(Value::Object(object))
            .map_err(D::Error::custom)?;
        Ok(Self {
            id,
            auth_token,
            command,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisServiceEndpointV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub address: SocketAddr,
    pub auth_token: String,
    pub workspace: PathBuf,
    pub process_id: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum OracleAnalysisServiceCommandV1 {
    Ping,
    Capabilities,
    Status {
        #[serde(default)]
        node: Option<usize>,
    },
    Explain {
        node: usize,
        owner_rank: u64,
    },
    View {
        #[serde(default)]
        node: Option<usize>,
    },
    Tree,
    Try {
        choice_ref: String,
    },
    Choose {
        node: usize,
        owner_rank: u64,
    },
    ChoosePath {
        node: usize,
        candidate_ids: Vec<String>,
    },
    Focus {
        node: usize,
    },
    Follow {
        edge: u64,
    },
    Back,
    Promote,
    Advance {
        #[serde(default = "default_max_quanta")]
        max_quanta: usize,
        #[serde(default = "default_quantum_nodes")]
        quantum_nodes: usize,
        #[serde(default = "default_quantum_ms")]
        quantum_ms: u64,
        #[serde(default)]
        wall_ms: Option<u64>,
    },
    AcceptCombat,
    RestartCombat,
    History {
        #[serde(default)]
        node: Option<usize>,
    },
    Journal {
        node: usize,
        #[serde(default = "default_journal_tail")]
        tail: usize,
    },
    Timeline {
        node: usize,
        #[serde(default = "default_journal_tail")]
        tail: usize,
    },
    JournalEntry {
        node: usize,
        index: usize,
    },
    Trajectory {
        node: usize,
    },
    CombatSummary {
        node: usize,
    },
    CombatDiagnostic {
        node: usize,
        #[serde(default = "default_max_engine_steps_per_transition")]
        max_engine_steps_per_transition: usize,
    },
    ExportCombatCase {
        node: usize,
        path: PathBuf,
    },
    ExportContinuation {
        node: usize,
        path: PathBuf,
    },
    Save,
    Shutdown,
}

fn default_max_engine_steps_per_transition() -> usize {
    512
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisServiceResponseV1 {
    pub protocol: String,
    pub protocol_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub event: String,
    pub ok: bool,
    pub revision: u64,
    pub saved_revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn call_oracle_analysis_tcp_v1(
    endpoint_path: &Path,
    request_json: &str,
) -> Result<OracleAnalysisServiceResponseV1, String> {
    let bytes = fs::read(endpoint_path).map_err(|error| {
        format!(
            "failed to read oracle service endpoint '{}': {error}",
            endpoint_path.display()
        )
    })?;
    let endpoint =
        serde_json::from_slice::<OracleAnalysisServiceEndpointV1>(&bytes).map_err(|error| {
            format!(
                "failed to parse oracle service endpoint '{}': {error}",
                endpoint_path.display()
            )
        })?;
    validate_endpoint(&endpoint)?;
    let mut request = serde_json::from_str::<Value>(request_json)
        .map_err(|error| format!("invalid oracle service request JSON: {error}"))?;
    let object = request
        .as_object_mut()
        .ok_or_else(|| "oracle service request must be a JSON object".to_string())?;
    object.insert("auth_token".to_string(), json!(endpoint.auth_token));

    let mut stream = TcpStream::connect(endpoint.address).map_err(|error| {
        format!(
            "failed to connect to oracle service at {}: {error}",
            endpoint.address
        )
    })?;
    serde_json::to_writer(&mut stream, &request)
        .map_err(|error| format!("failed to serialize oracle service request: {error}"))?;
    stream
        .write_all(b"\n")
        .map_err(|error| format!("failed to write oracle service request: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("failed to flush oracle service request: {error}"))?;
    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .map_err(|error| format!("failed to read oracle service response: {error}"))?;
    serde_json::from_str::<OracleAnalysisServiceResponseV1>(&response)
        .map_err(|error| format!("failed to parse oracle service response: {error}"))
}

pub fn validate_endpoint(endpoint: &OracleAnalysisServiceEndpointV1) -> Result<(), String> {
    if endpoint.schema_name != ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA
        || endpoint.schema_version != ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA_VERSION
    {
        return Err("unsupported oracle analysis service endpoint schema".to_string());
    }
    if !endpoint.address.ip().is_loopback() {
        return Err(format!(
            "oracle analysis endpoint is not loopback-only: {}",
            endpoint.address
        ));
    }
    Ok(())
}

const fn default_max_quanta() -> usize {
    1
}
const fn default_quantum_nodes() -> usize {
    50_000
}
const fn default_quantum_ms() -> u64 {
    1_000
}
const fn default_journal_tail() -> usize {
    32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loopback_call_injects_endpoint_auth_without_exposing_it_to_the_caller() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback test server");
        let address = listener.local_addr().expect("inspect test address");
        let endpoint_path = std::env::temp_dir().join(format!(
            "oracle-lab-protocol-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let endpoint = OracleAnalysisServiceEndpointV1 {
            schema_name: ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA.to_string(),
            schema_version: ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA_VERSION,
            address,
            auth_token: "secret-token".to_string(),
            workspace: PathBuf::from("workspace.json"),
            process_id: std::process::id(),
        };
        fs::write(
            &endpoint_path,
            serde_json::to_vec(&endpoint).expect("serialize endpoint"),
        )
        .expect("write endpoint");

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept client");
            let mut connection = BufReader::new(stream);
            let mut request = String::new();
            connection.read_line(&mut request).expect("read request");
            let request: Value = serde_json::from_str(&request).expect("parse request");
            assert_eq!(request.get("command"), Some(&json!("ping")));
            assert_eq!(request.get("auth_token"), Some(&json!("secret-token")));
            let response = OracleAnalysisServiceResponseV1 {
                protocol: ORACLE_ANALYSIS_SERVICE_PROTOCOL.to_string(),
                protocol_version: ORACLE_ANALYSIS_SERVICE_PROTOCOL_VERSION,
                id: None,
                event: "pong".to_string(),
                ok: true,
                revision: 0,
                saved_revision: 0,
                result: Some(json!({"alive": true})),
                error: None,
            };
            serde_json::to_writer(connection.get_mut(), &response).expect("write response");
            connection
                .get_mut()
                .write_all(b"\n")
                .expect("finish response");
        });

        let response = call_oracle_analysis_tcp_v1(&endpoint_path, r#"{"command":"ping"}"#)
            .expect("call loopback service");
        assert!(response.ok);
        assert_eq!(response.result, Some(json!({"alive": true})));
        server.join().expect("join test server");
        let _ = fs::remove_file(endpoint_path);
    }

    #[test]
    fn endpoint_validation_rejects_non_loopback_addresses() {
        let endpoint = OracleAnalysisServiceEndpointV1 {
            schema_name: ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA.to_string(),
            schema_version: ORACLE_ANALYSIS_SERVICE_ENDPOINT_SCHEMA_VERSION,
            address: "192.0.2.1:1234".parse().expect("static address"),
            auth_token: "unused".to_string(),
            workspace: PathBuf::new(),
            process_id: 0,
        };
        assert!(validate_endpoint(&endpoint).is_err());
    }

    #[test]
    fn combat_diagnostic_uses_a_bounded_transition_default() {
        let command = serde_json::from_value::<OracleAnalysisServiceCommandV1>(json!({
            "command": "combat_diagnostic",
            "node": 17,
        }))
        .expect("parse diagnostic command");
        assert!(matches!(
            command,
            OracleAnalysisServiceCommandV1::CombatDiagnostic {
                node: 17,
                max_engine_steps_per_transition: 512,
            }
        ));
    }
}
