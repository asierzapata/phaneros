//
//  PhanerosClient.swift
//  phaneros
//
//  The seam between the UI and the engine.
//
//  documentation/desktop-and-mobile-clients.md argues the socket protocol is the
//  portable artifact, and that the schema is the part worth getting right. So the UI
//  talks to this protocol and nothing else. Today the only implementation is
//  `DemoClient`; when phaneros-daemon exists, a `DaemonClient` conforming to the same
//  protocol slots in at the one call site in `PhanerosApp` and no view changes.
//

import Foundation

/// Mirrors the `Command` enum from the RFC.
enum Command: Equatable, Sendable {
    case listDrives
    case addDrive(name: String, path: URL, storeURL: String, driveID: String)
    case removeDrive(driveID: String)
    case pause(driveID: String)
    case resume(driveID: String)
    case pauseAll
    case resumeAll
    case setToken(storeURL: String, token: String)
    case resetSyncState(driveID: String)
}

/// Mirrors the `Event` enum from the RFC.
enum Event: Equatable, Sendable {
    case driveStatus(driveID: String, status: DriveStatus)
    case syncProgress(driveID: String, progress: FirstSyncProgress)
    case firstSyncFinished(driveID: String, fileCount: Int)
    case conflictWritten(driveID: String, conflict: Conflict)
    case activity(driveID: String, entry: ActivityEntry)
    case storeUnreachable(host: String, since: Date)
    case storeReachable(host: String)
    case tokenRejected(host: String)
    case drivesChanged([Drive])
}

protocol PhanerosClient: AnyObject, Sendable {
    /// A long-lived stream, equivalent to `Subscribe` on the socket.
    func events() -> AsyncStream<Event>
    func send(_ command: Command) async
}
