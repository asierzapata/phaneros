//
//  DriveStatus.swift
//  phaneros
//
//  DriveStatus enum representing the sync status of a drive.
//

import Foundation

enum DriveStatus: Equatable, Hashable, Sendable {
    case upToDate(at: Date)
    case working
    case paused
    case needsAttention(reason: String)
    case offline

    nonisolated static func == (lhs: DriveStatus, rhs: DriveStatus) -> Bool {
        switch (lhs, rhs) {
        case (.upToDate(let a), .upToDate(let b)): a == b
        case (.working, .working): true
        case (.paused, .paused): true
        case (.needsAttention(let a), .needsAttention(let b)): a == b
        case (.offline, .offline): true
        default: false
        }
    }

    nonisolated func hash(into hasher: inout Hasher) {
        switch self {
        case .upToDate(let at): hasher.combine(0); hasher.combine(at)
        case .working: hasher.combine(1)
        case .paused: hasher.combine(2)
        case .needsAttention(let reason): hasher.combine(3); hasher.combine(reason)
        case .offline: hasher.combine(4)
        }
    }

    /// One-to-one with the mark. If the engine can't name a state, the icon can't show it.
    var mark: MarkState {
        switch self {
        case .upToDate: .upToDate
        case .working: .syncing
        case .paused: .paused
        case .needsAttention: .attention
        case .offline: .offline
        }
    }

    /// The phrase that appears under a drive's name. Written the way a person would say it.
    var label: String {
        switch self {
        case .upToDate(let at): "Synced \(at.phanerosRelative)"
        case .working: "Syncing…"
        case .paused: "Paused"
        case .needsAttention: "Needs a look"
        case .offline: "Can't reach the store"
        }
    }

    /// The shorter form used in the sidebar, where the row is already narrow.
    var shortLabel: String {
        switch self {
        case .upToDate: "Up to date"
        case .working: "Syncing…"
        case .paused: "Paused"
        case .needsAttention: "Needs attention"
        case .offline: "Can't reach the store"
        }
    }
}
