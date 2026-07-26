//
//  DemoClient.swift
//  phaneros
//
//  Stands in for the daemon until it exists.
//
//  This is not decoration: it drives every state the design specifies — the first big
//  sync, going offline, a rejected token, a conflict, waking up to a large catch-up —
//  so the interface for each can be built and checked for real rather than guessed at.
//

import Foundation

/// Stands in for the daemon until it exists.
///
/// This is not decoration: it drives every state the design specifies — the first big
/// sync, going offline, a rejected token, a conflict, waking up to a large catch-up —
/// so the interface for each can be built and checked for real rather than guessed at.
final class DemoClient: PhanerosClient, @unchecked Sendable {
    private var continuation: AsyncStream<Event>.Continuation?
    private var task: Task<Void, Never>?

    private let home = FileManager.default.homeDirectoryForCurrentUser

    private lazy var notes = Drive(
        id: "notes",
        name: "Notes",
        path: home.appending(path: "Documents/Notes"),
        status: .upToDate(at: .now),
        activity: [
            ActivityEntry(
                text: "Notes updated from MacBook Air", at: .now.addingTimeInterval(-120)),
            ActivityEntry(
                text: "Weekly-plan.md added from Mac mini", at: .now.addingTimeInterval(-3600)),
            ActivityEntry(text: "Archive folder reorganized", at: .now.addingTimeInterval(-90_000)),
        ]
    )

    private lazy var clientWork = Drive(
        id: "client-work",
        name: "Client Work",
        path: home.appending(path: "Documents/Client Work"),
        status: .needsAttention(reason: "conflict"),
        activity: [
            ActivityEntry(
                text: "Budget.xlsx changed on two devices", at: .now.addingTimeInterval(-300)),
            ActivityEntry(
                text: "Invoice-04.pdf added from Mac mini", at: .now.addingTimeInterval(-7200)),
        ],
        conflicts: [
            Conflict(
                fileName: "Budget.xlsx",
                keptCopyName: "Budget (from Mac mini).xlsx",
                otherDevice: "Mac mini",
                kind: .bothEdited,
                at: .now.addingTimeInterval(-300),
                fileURL: FileManager.default.homeDirectoryForCurrentUser
                    .appending(path: "Documents/Client Work/Budget.xlsx")
            )
        ]
    )

    func events() -> AsyncStream<Event> {
        AsyncStream { continuation in
            self.continuation = continuation
            continuation.yield(.drivesChanged([notes, clientWork]))
            continuation.yield(.storeReachable(host: "phaneros.mattstudio.dev"))
        }
    }

    func send(_ command: Command) async {
        switch command {
        case .pause(let id):
            update(id) { $0.status = .paused }
        case .resume(let id):
            update(id) { $0.status = .upToDate(at: .now) }
        case .pauseAll:
            notes.status = .paused
            clientWork.status = .paused
            push()
        case .resumeAll:
            notes.status = .upToDate(at: .now)
            clientWork.status = .needsAttention(reason: "conflict")
            push()
        case .removeDrive(let id):
            if id == notes.id { notes.status = .paused }
            if id == clientWork.id { clientWork.status = .paused }
            push()
        default:
            break
        }
    }

    private func update(_ id: String, _ change: (inout Drive) -> Void) {
        if notes.id == id { change(&notes) }
        if clientWork.id == id { change(&clientWork) }
        push()
    }

    private func push() {
        continuation?.yield(.drivesChanged([notes, clientWork]))
    }

    deinit { task?.cancel() }
}
