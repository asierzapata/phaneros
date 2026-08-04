use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteListenerEvent {
    pub id: i64,
    pub drive_id: String,
    pub root: String,
    pub at: i64,
}

#[derive(Debug, Deserialize)]
struct RootChangedPayload {
    drive_id: String,
    root: String,
    at: i64,
}

const RECONNECT_BACKOFF_STEPS: [Duration; 4] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
];

pub fn spawn_remote_listener<F>(
    base_url: String,
    drive_id: String,
    token: String,
    mut on_event: F,
) -> tokio::task::JoinHandle<()>
where
    F: FnMut(RemoteListenerEvent) + Send + 'static,
{
    tokio::spawn(async move {
        let events_url = format!(
            "{}/api/drives/{}/events",
            base_url.trim_end_matches('/'),
            drive_id
        );
        let auth = format!("Bearer {}", token);
        let client = reqwest::Client::builder()
            .build()
            .unwrap();
        let mut last_event_id: Option<i64> = None;
        let mut backoff_index = 0usize;

        loop {
            let mut request = client
                .get(&events_url)
                .header("Authorization", &auth)
                .header("Accept", "text/event-stream");

            if let Some(id) = last_event_id {
                request = request.header("Last-Event-ID", id.to_string());
            }

            match request.send().await {
                Ok(mut response) => {
                    backoff_index = 0;

                    let mut event_type = String::new();
                    let mut event_id: Option<i64> = None;
                    let mut data_lines: Vec<String> = Vec::new();
                    let mut buf = Vec::new();

                    while let Ok(Some(chunk)) = response.chunk().await {
                        buf.extend_from_slice(&chunk);

                        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                            let mut line = String::from_utf8_lossy(&buf[..pos]).to_string();
                            buf.drain(..pos + 1);

                            while line.ends_with('\n') || line.ends_with('\r') {
                                line.pop();
                            }

                            if line.is_empty() {
                                dispatch_event_frame(&event_type, event_id, &data_lines, &mut last_event_id, &mut on_event);
                                event_type.clear();
                                event_id = None;
                                data_lines.clear();
                                continue;
                            }

                            if line.starts_with(':') {
                                continue;
                            }

                            let (field, value) = match line.split_once(':') {
                                Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
                                None => (line.as_str(), ""),
                            };

                            match field {
                                "event" => {
                                    event_type.clear();
                                    event_type.push_str(value);
                                }
                                "id" => {
                                    event_id = value.parse::<i64>().ok().filter(|id| *id >= 0);
                                }
                                "data" => data_lines.push(value.to_string()),
                                _ => {}
                            }
                        }
                    }

                    eprintln!("SSE stream disconnected; reconnecting...");
                }
                Err(err) => {
                    eprintln!("SSE connect failed: {err}");
                }
            }

            let sleep_for = RECONNECT_BACKOFF_STEPS[backoff_index];
            if backoff_index + 1 < RECONNECT_BACKOFF_STEPS.len() {
                backoff_index += 1;
            }
            tokio::time::sleep(sleep_for).await;
        }
    })
}

pub fn parse_event_stream<R, F>(reader: R, last_event_id: &mut Option<i64>, mut on_event: F)
where
    R: std::io::BufRead,
    F: FnMut(RemoteListenerEvent),
{
    let mut event_type = String::new();
    let mut event_id = None;
    let mut data_lines: Vec<String> = Vec::new();

    for line in reader.lines().flatten() {
        if line.is_empty() {
            dispatch_event_frame(
                &event_type,
                event_id,
                &data_lines,
                last_event_id,
                &mut on_event,
            );
            event_type.clear();
            event_id = None;
            data_lines.clear();
            continue;
        }

        if line.starts_with(':') {
            continue;
        }

        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line.as_str(), ""),
        };

        match field {
            "event" => {
                event_type.clear();
                event_type.push_str(value);
            }
            "id" => {
                event_id = value.parse::<i64>().ok().filter(|id| *id >= 0);
            }
            "data" => data_lines.push(value.to_string()),
            _ => {}
        }
    }

    if !event_type.is_empty() || !data_lines.is_empty() {
        dispatch_event_frame(
            &event_type,
            event_id,
            &data_lines,
            last_event_id,
            &mut on_event,
        );
    }
}

fn dispatch_event_frame<F>(
    event_type: &str,
    event_id: Option<i64>,
    data_lines: &[String],
    last_event_id: &mut Option<i64>,
    on_event: &mut F,
) where
    F: FnMut(RemoteListenerEvent),
{
    if event_type != "root-changed" || data_lines.is_empty() {
        return;
    }

    let Some(id) = event_id else {
        return;
    };

    let payload_raw = data_lines.join("\n");
    let payload: RootChangedPayload = match serde_json::from_str(&payload_raw) {
        Ok(payload) => payload,
        Err(err) => {
            eprintln!("SSE payload decode failed: {err}");
            return;
        }
    };

    *last_event_id = Some(id);
    on_event(RemoteListenerEvent {
        id,
        drive_id: payload.drive_id,
        root: payload.root,
        at: payload.at,
    });
}
