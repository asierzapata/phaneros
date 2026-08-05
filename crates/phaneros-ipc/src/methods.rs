use phaneros_core::telemetry::{DriveStatus, SyncProgressEvent};
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveIdParams {
    pub drive_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsParams {
    #[serde(default)]
    pub drive_id: Option<String>,
}

fn default_activity_limit() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityListParams {
    #[serde(default)]
    pub drive_id: Option<String>,
    #[serde(default = "default_activity_limit")]
    pub limit: usize,
}

impl Default for ActivityListParams {
    fn default() -> Self {
        Self {
            drive_id: None,
            limit: default_activity_limit(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDriveParams {
    pub drive_id: String,
    pub path: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub store_url: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveSummary {
    pub drive_id: String,
    pub path: String,
    pub store_url: String,
    pub enabled: bool,
    pub status: DriveStatus,
    pub last_synced_root: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveStatusResult {
    #[serde(flatten)]
    pub summary: DriveSummary,
    pub progress: Option<SyncProgressEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub version: String,
    pub pid: u32,
    pub uptime_seconds: u64,
    pub configured: bool,
}

/// One variant per JSON-RPC method a client can call. The derived serde
/// representation (`#[serde(tag = "method", content = "params")]`) is
/// structurally identical to the wire `{ "method": ..., "params": ... }`
/// shape, so [`Request::into_parts`]/[`Request::from_parts`] convert to and
/// from a bare `JsonRpcRequest` without a second hand-written mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Request {
    #[serde(rename = "daemon.ping")]
    DaemonPing,
    #[serde(rename = "daemon.shutdown")]
    DaemonShutdown,
    #[serde(rename = "drives.list")]
    DrivesList,
    #[serde(rename = "drives.status")]
    DrivesStatus(DriveIdParams),
    #[serde(rename = "drives.start")]
    DrivesStart(DriveIdParams),
    #[serde(rename = "drives.stop")]
    DrivesStop(DriveIdParams),
    #[serde(rename = "drives.add")]
    DrivesAdd(AddDriveParams),
    #[serde(rename = "drives.remove")]
    DrivesRemove(DriveIdParams),
    #[serde(rename = "drives.trigger_sync")]
    DrivesTriggerSync(DriveIdParams),
    #[serde(rename = "config.reload")]
    ConfigReload,
    #[serde(rename = "stats.aggregate")]
    StatsAggregate(StatsParams),
    #[serde(rename = "activity.list")]
    ActivityList(ActivityListParams),
    #[serde(rename = "events.subscribe")]
    EventsSubscribe,
}

impl Request {
    pub fn into_parts(self) -> (String, Option<Value>) {
        let value = serde_json::to_value(&self).expect("Request always serializes");
        let method = value["method"]
            .as_str()
            .expect("tagged enum always has a method field")
            .to_string();
        let params = value.get("params").cloned();
        (method, params)
    }

    pub fn from_parts(method: &str, params: Option<Value>) -> Result<Self, serde_json::Error> {
        let mut object = serde_json::Map::new();
        object.insert("method".to_string(), Value::String(method.to_string()));
        if let Some(params) = params {
            object.insert("params".to_string(), params);
        }
        serde_json::from_value(Value::Object(object))
    }
}

/// Server-to-client push events, sent only to connections that have called
/// `events.subscribe`. Uses the same adjacently-tagged encoding as [`Request`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Notification {
    #[serde(rename = "event.progress")]
    EventProgress {
        drive_id: String,
        event: SyncProgressEvent,
    },
    #[serde(rename = "event.drive_status_changed")]
    EventDriveStatusChanged {
        drive_id: String,
        status: DriveStatus,
    },
}

impl Notification {
    pub fn into_parts(self) -> (String, Option<Value>) {
        let value = serde_json::to_value(&self).expect("Notification always serializes");
        let method = value["method"]
            .as_str()
            .expect("tagged enum always has a method field")
            .to_string();
        let params = value.get("params").cloned();
        (method, params)
    }

    pub fn from_parts(method: &str, params: Option<Value>) -> Result<Self, serde_json::Error> {
        let mut object = serde_json::Map::new();
        object.insert("method".to_string(), Value::String(method.to_string()));
        if let Some(params) = params {
            object.insert("params".to_string(), params);
        }
        serde_json::from_value(Value::Object(object))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_into_parts_matches_wire_shape() {
        let (method, params) = Request::DrivesStart(DriveIdParams {
            drive_id: "default".to_string(),
        })
        .into_parts();
        assert_eq!(method, "drives.start");
        assert_eq!(params.unwrap()["drive_id"], "default");
    }

    #[test]
    fn request_round_trips_through_parts() {
        let original = Request::DrivesAdd(AddDriveParams {
            drive_id: "work".to_string(),
            path: "/tmp/work".to_string(),
            token: Some("tok".to_string()),
            store_url: None,
            enabled: true,
        });
        let (method, params) = original.clone().into_parts();
        let parsed = Request::from_parts(&method, params).unwrap();
        assert_eq!(
            serde_json::to_string(&original).unwrap(),
            serde_json::to_string(&parsed).unwrap()
        );
    }

    #[test]
    fn unit_variant_has_no_params() {
        let (method, params) = Request::DrivesList.into_parts();
        assert_eq!(method, "drives.list");
        assert!(params.is_none());
    }

    #[test]
    fn activity_list_round_trips_through_parts() {
        let original = Request::ActivityList(ActivityListParams {
            drive_id: Some("default".to_string()),
            limit: 10,
        });
        let (method, params) = original.clone().into_parts();
        assert_eq!(method, "activity.list");
        let parsed = Request::from_parts(&method, params).unwrap();
        assert_eq!(
            serde_json::to_string(&original).unwrap(),
            serde_json::to_string(&parsed).unwrap()
        );
    }

    #[test]
    fn activity_list_params_default_limit() {
        let params = ActivityListParams::default();
        assert_eq!(params.limit, 20);
        assert!(params.drive_id.is_none());
    }

    #[test]
    fn notification_round_trips_through_parts() {
        let original = Notification::EventDriveStatusChanged {
            drive_id: "default".to_string(),
            status: DriveStatus::Reconciling,
        };
        let (method, params) = original.clone().into_parts();
        assert_eq!(method, "event.drive_status_changed");
        let parsed = Notification::from_parts(&method, params).unwrap();
        assert_eq!(
            serde_json::to_string(&original).unwrap(),
            serde_json::to_string(&parsed).unwrap()
        );
    }
}
