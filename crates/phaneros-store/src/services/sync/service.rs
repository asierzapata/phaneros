use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::Serialize;
use tokio::{
    sync::broadcast,
    time::{self, MissedTickBehavior},
};

use crate::services::node::{NodeService, VersionEvent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RootChanged {
    pub id: i64,
    pub drive_id: String,
    pub root: String,
    pub at: i64,
}

impl From<VersionEvent> for RootChanged {
    fn from(value: VersionEvent) -> Self {
        Self {
            id: value.id,
            drive_id: value.drive_id,
            root: value.root,
            at: value.at,
        }
    }
}

#[derive(Default)]
struct SeenIds {
    queue: VecDeque<i64>,
    set: HashSet<i64>,
}

impl SeenIds {
    fn remember(&mut self, id: i64, window: usize) -> bool {
        if self.set.contains(&id) {
            return false;
        }

        self.queue.push_back(id);
        self.set.insert(id);

        while self.queue.len() > window {
            if let Some(removed) = self.queue.pop_front() {
                self.set.remove(&removed);
            }
        }

        true
    }
}

struct SyncServiceInner {
    subscribers: Mutex<HashMap<String, broadcast::Sender<RootChanged>>>,
    seen_ids: Mutex<SeenIds>,
    channel_capacity: usize,
    dedupe_window: usize,
}

#[derive(Clone)]
pub struct SyncService {
    inner: Arc<SyncServiceInner>,
}

impl Default for SyncService {
    fn default() -> Self {
        Self::new(256, 4096)
    }
}

impl SyncService {
    pub fn new(channel_capacity: usize, dedupe_window: usize) -> Self {
        Self {
            inner: Arc::new(SyncServiceInner {
                subscribers: Mutex::new(HashMap::new()),
                seen_ids: Mutex::new(SeenIds::default()),
                channel_capacity,
                dedupe_window: dedupe_window.max(1),
            }),
        }
    }

    pub fn subscribe(&self, drive_id: &str) -> broadcast::Receiver<RootChanged> {
        let mut subscribers = self
            .inner
            .subscribers
            .lock()
            .expect("sync subscriber registry poisoned");

        let sender = subscribers
            .entry(drive_id.to_string())
            .or_insert_with(|| broadcast::channel(self.inner.channel_capacity).0);

        sender.subscribe()
    }

    pub fn publish(&self, event: RootChanged) {
        {
            let mut seen = self
                .inner
                .seen_ids
                .lock()
                .expect("sync dedupe state poisoned");
            if !seen.remember(event.id, self.inner.dedupe_window) {
                return;
            }
        }

        let sender = {
            let subscribers = self
                .inner
                .subscribers
                .lock()
                .expect("sync subscriber registry poisoned");
            subscribers.get(&event.drive_id).cloned()
        };

        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    }

    pub fn publish_version(&self, version: VersionEvent) {
        self.publish(version.into());
    }

    pub async fn run_versions_poller(
        &self,
        node_service: NodeService,
        poll_interval: Duration,
        batch_size: i64,
    ) {
        let batch_size = batch_size.max(1);

        let mut last_seen_version_id = match node_service.max_version_id().await {
            Ok(id) => id,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "failed to initialize versions poller watermark; starting from id=0"
                );
                0
            }
        };

        let mut interval = time::interval(poll_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            loop {
                let versions = match node_service
                    .list_versions_after(last_seen_version_id, batch_size)
                    .await
                {
                    Ok(versions) => versions,
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            last_seen_version_id,
                            "versions poller query failed"
                        );
                        break;
                    }
                };

                if versions.is_empty() {
                    break;
                }

                for version in versions.iter() {
                    last_seen_version_id = version.id;
                    self.publish_version(version.clone());
                }

                if versions.len() < batch_size as usize {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: i64, drive_id: &str, root: &str) -> RootChanged {
        RootChanged {
            id,
            drive_id: drive_id.to_string(),
            root: root.to_string(),
            at: 1,
        }
    }

    #[tokio::test]
    async fn subscribers_only_receive_events_for_their_drive() {
        let sync_service = SyncService::new(8, 64);
        let mut drive_a = sync_service.subscribe("drive-a");
        let mut drive_b = sync_service.subscribe("drive-b");

        sync_service.publish(event(1, "drive-a", "root-a"));

        let got_a = drive_a.recv().await.unwrap();
        assert_eq!(got_a.drive_id, "drive-a");
        assert!(drive_b.try_recv().is_err());
    }

    #[tokio::test]
    async fn duplicate_event_ids_are_deduped() {
        let sync_service = SyncService::new(8, 64);
        let mut rx = sync_service.subscribe("drive-a");

        sync_service.publish(event(5, "drive-a", "root-1"));
        sync_service.publish(event(5, "drive-a", "root-1"));

        let first = rx.recv().await.unwrap();
        assert_eq!(first.id, 5);
        assert!(rx.try_recv().is_err());
    }
}
