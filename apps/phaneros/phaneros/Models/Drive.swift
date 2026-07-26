//
//  Drive.swift
//  phaneros
//
//  Drive model with its associated FirstSyncProgress.
//

import Foundation

struct Drive: Identifiable, Equatable, Hashable, @unchecked Sendable {
    let id: String
    var name: String
    var path: URL
    var status: DriveStatus
    var activity: [ActivityEntry] = []
    var conflicts: [Conflict] = []
    /// Present only during the first sync of a folder — the one earned progress bar.
    var firstSync: FirstSyncProgress?

    /// `~/Documents/Notes` rather than the absolute path. The design shows the tilde form.
    var displayPath: String {
        // Directory URLs carry a trailing slash and file ones don't, so both sides get
        // trimmed before comparing — otherwise the home folder itself renders as "~/".
        func trimmed(_ url: URL) -> String {
            let text = url.path(percentEncoded: false)
            return text.count > 1 && text.hasSuffix("/") ? String(text.dropLast()) : text
        }
        let home = trimmed(FileManager.default.homeDirectoryForCurrentUser)
        let full = trimmed(path)
        guard full == home || full.hasPrefix(home + "/") else { return full }
        return "~" + full.dropFirst(home.count)
    }

    nonisolated static func == (lhs: Drive, rhs: Drive) -> Bool {
        lhs.id == rhs.id && lhs.name == rhs.name && lhs.path == rhs.path
            && lhs.status == rhs.status && lhs.activity == rhs.activity
            && lhs.conflicts == rhs.conflicts && lhs.firstSync == rhs.firstSync
    }

    nonisolated func hash(into hasher: inout Hasher) {
        hasher.combine(id)
        hasher.combine(name)
        hasher.combine(path)
        hasher.combine(status)
        hasher.combine(activity)
        hasher.combine(conflicts)
        hasher.combine(firstSync)
    }
}

struct FirstSyncProgress: Equatable, Hashable, Sendable {
    var filesDone: Int
    var filesTotal: Int
    var estimatedRemaining: TimeInterval?

    var fraction: Double {
        guard filesTotal > 0 else { return 0 }
        return min(1, Double(filesDone) / Double(filesTotal))
    }

    /// "3,800 of 10,000 files · about 12 minutes left"
    var caption: String {
        let done = filesDone.formatted(.number)
        let total = filesTotal.formatted(.number)
        var text = "\(done) of \(total) files"
        if let remaining = estimatedRemaining, remaining > 0 {
            text += " · about \(remaining.phanerosCoarseDuration) left"
        }
        return text
    }

    nonisolated static func == (lhs: FirstSyncProgress, rhs: FirstSyncProgress) -> Bool {
        lhs.filesDone == rhs.filesDone && lhs.filesTotal == rhs.filesTotal
            && lhs.estimatedRemaining == rhs.estimatedRemaining
    }

    nonisolated func hash(into hasher: inout Hasher) {
        hasher.combine(filesDone)
        hasher.combine(filesTotal)
        hasher.combine(estimatedRemaining)
    }
}
