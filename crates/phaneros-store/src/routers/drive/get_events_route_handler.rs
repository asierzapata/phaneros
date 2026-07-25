use std::{convert::Infallible, time::Duration};

use async_stream::stream;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};

use crate::{services::sync::RootChanged, state::AppState};

const REPLAY_BATCH_SIZE: i64 = 500;

pub async fn get_events(
    State(state): State<AppState>,
    Path(drive_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let replay_after_id = match parse_last_event_id(&headers) {
        Ok(value) => value,
        Err(()) => {
            return (
                StatusCode::BAD_REQUEST,
                "invalid Last-Event-ID header; expected a non-negative integer",
            )
                .into_response();
        }
    };

    // Subscribe first so events published during replay are queued, then replay
    // from durable storage and finally drain live events.
    let mut live_rx = state.sync_service.subscribe(&drive_id);
    let node_service = state.node_service.clone();

    let stream_drive_id = drive_id.clone();
    let event_stream = stream! {
        let mut last_emitted_id = replay_after_id.unwrap_or(0);

        if let Some(mut replay_cursor) = replay_after_id {
            loop {
                let replay_batch = match node_service
                    .list_drive_versions_after(&stream_drive_id, replay_cursor, REPLAY_BATCH_SIZE)
                    .await
                {
                    Ok(batch) => batch,
                    Err(err) => {
                        tracing::warn!(
                            drive_id = %stream_drive_id,
                            replay_cursor,
                            error = %err,
                            "failed to replay events"
                        );
                        return;
                    }
                };

                if replay_batch.is_empty() {
                    break;
                }

                for version in replay_batch.iter() {
                    replay_cursor = version.id;
                    last_emitted_id = version.id;

                    let event = RootChanged::from(version.clone());
                    if let Some(frame) = to_sse_frame(&event) {
                        yield Ok::<Event, Infallible>(frame);
                    }
                }

                if replay_batch.len() < REPLAY_BATCH_SIZE as usize {
                    break;
                }
            }
        }

        loop {
            match live_rx.recv().await {
                Ok(event) => {
                    if event.id <= last_emitted_id {
                        continue;
                    }
                    last_emitted_id = event.id;

                    if let Some(frame) = to_sse_frame(&event) {
                        yield Ok::<Event, Infallible>(frame);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(
                        drive_id = %stream_drive_id,
                        skipped,
                        "SSE subscriber lagged behind; closing stream so client can replay with Last-Event-ID"
                    );
                    return;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return;
                }
            }
        }
    };

    Sse::new(event_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(20))
                .text("keepalive"),
        )
        .into_response()
}

fn parse_last_event_id(headers: &HeaderMap) -> Result<Option<i64>, ()> {
    let Some(raw) = headers.get("last-event-id") else {
        return Ok(None);
    };

    let value = raw.to_str().map_err(|_| ())?;
    let parsed = value.parse::<i64>().map_err(|_| ())?;
    if parsed < 0 {
        return Err(());
    }

    Ok(Some(parsed))
}

#[derive(serde::Serialize)]
struct RootChangedPayloadWire<'a> {
    drive_id: &'a str,
    root: &'a str,
    at: i64,
}

fn to_sse_frame(event: &RootChanged) -> Option<Event> {
    let data = match serde_json::to_string(&RootChangedPayloadWire {
        drive_id: &event.drive_id,
        root: &event.root,
        at: event.at,
    }) {
        Ok(data) => data,
        Err(err) => {
            tracing::warn!(
                id = event.id,
                drive_id = %event.drive_id,
                error = %err,
                "failed to serialize root-changed event"
            );
            return None;
        }
    };

    Some(
        Event::default()
            .id(event.id.to_string())
            .event("root-changed")
            .data(data),
    )
}

#[cfg(test)]
mod tests {
    use super::parse_last_event_id;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn parses_absent_last_event_id() {
        let headers = HeaderMap::new();
        assert_eq!(parse_last_event_id(&headers).unwrap(), None);
    }

    #[test]
    fn parses_numeric_last_event_id() {
        let mut headers = HeaderMap::new();
        headers.insert("last-event-id", HeaderValue::from_static("42"));

        assert_eq!(parse_last_event_id(&headers).unwrap(), Some(42));
    }

    #[test]
    fn rejects_invalid_last_event_id() {
        let mut headers = HeaderMap::new();
        headers.insert("last-event-id", HeaderValue::from_static("not-a-number"));

        assert!(parse_last_event_id(&headers).is_err());
    }
}
