//
//  ActivityEntry.swift
//  phaneros
//
//  ActivityEntry model representing a sync activity log entry.
//

import Foundation

struct ActivityEntry: Identifiable, Equatable, Hashable, Sendable {
    let id = UUID()
    /// Already phrased as a sentence: "Notes updated from MacBook Air".
    var text: String
    var at: Date

    /// "Notes updated from MacBook Air — 2 min ago"
    var line: String { "\(text) — \(at.phanerosRelative)" }

    nonisolated static func == (lhs: ActivityEntry, rhs: ActivityEntry) -> Bool {
        lhs.id == rhs.id && lhs.text == rhs.text && lhs.at == rhs.at
    }

    nonisolated func hash(into hasher: inout Hasher) {
        hasher.combine(id)
        hasher.combine(text)
        hasher.combine(at)
    }
}
