use crate::watcher::remote_listener::parse_event_stream;

#[test]
fn parses_remote_listener_event_and_tracks_last_id() {
    let input = concat!(
        "id: 42\n",
        "event: root-changed\n",
        "data: {\"drive_id\":\"default\",\"root\":\"abc123\",\"at\":1721920000}\n",
        "\n"
    );

    let mut received = Vec::new();
    let mut last_event_id = None;

    parse_event_stream(input.as_bytes(), &mut last_event_id, &mut |event| {
        received.push(event);
    });

    assert_eq!(last_event_id, Some(42));
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].drive_id, "default");
    assert_eq!(received[0].root, "abc123");
}

#[test]
fn ignores_non_root_changed_events_and_invalid_ids() {
    let input = concat!(
        "id: nope\n",
        "event: root-changed\n",
        "data: {\"drive_id\":\"default\",\"root\":\"abc123\",\"at\":1721920000}\n",
        "\n",
        "id: 7\n",
        "event: ping\n",
        "data: {}\n",
        "\n"
    );

    let mut received = Vec::new();
    let mut last_event_id = None;

    parse_event_stream(input.as_bytes(), &mut last_event_id, &mut |event| {
        received.push(event);
    });

    assert!(received.is_empty());
    assert_eq!(last_event_id, None);
}
