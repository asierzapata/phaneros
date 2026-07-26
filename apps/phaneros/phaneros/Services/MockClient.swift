//
//  MockClient.swift
//  phaneros
//
//  A controllable mock of PhanerosClient for testing.
//  Yields events on demand and records every command sent.
//

import Foundation

/// A controllable mock implementation of `PhanerosClient` for testing.
///
/// Usage:
/// ```swift
/// let client = MockClient()
///
/// // Push events into the stream
/// client.sendEvent(.drivesChanged([mockDrive]))
///
/// // Assert commands were received
/// client.send(.pause(driveID: "test"))
/// XCTAssertEqual(client.commandsReceived, [.pause(driveID: "test")])
/// ```
final class MockClient: PhanerosClient, @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: AsyncStream<Event>.Continuation?

    /// Every command sent via `send(_:)`, in order.
    private(set) var commandsReceived: [Command] = []

    /// Optional handler called for each command, allowing tests to trigger side effects
    /// (e.g. push events in response to a command).
    var onCommand: ((Command) -> Void)?

    func events() -> AsyncStream<Event> {
        AsyncStream { continuation in
            self.lock.lock()
            self.continuation = continuation
            self.lock.unlock()
        }
    }

    /// Push a single event into the stream.
    func sendEvent(_ event: Event) {
        lock.lock()
        let continuation = self.continuation
        lock.unlock()
        continuation?.yield(event)
    }

    /// Push multiple events into the stream.
    func sendEvents(_ events: [Event]) {
        for event in events {
            sendEvent(event)
        }
    }

    func send(_ command: Command) async {
        lock.lock()
        commandsReceived.append(command)
        let handler = self.onCommand
        lock.unlock()
        handler?(command)
    }

    /// Reset recorded commands and event continuation.
    func reset() {
        lock.lock()
        commandsReceived.removeAll()
        continuation = nil
        lock.unlock()
    }
}
