//
//  Conflict.swift
//  phaneros
//
//  Conflict model representing a sync conflict between devices.
//

import Foundation

struct Conflict: Identifiable, Equatable, Hashable, Sendable {
    enum Kind: Equatable, Hashable, Sendable {
        case bothEdited
        case editedAndDeleted

        nonisolated static func == (lhs: Kind, rhs: Kind) -> Bool {
            switch (lhs, rhs) {
            case (.bothEdited, .bothEdited): true
            case (.editedAndDeleted, .editedAndDeleted): true
            default: false
            }
        }

        nonisolated func hash(into hasher: inout Hasher) {
            switch self {
            case .bothEdited: hasher.combine(0)
            case .editedAndDeleted: hasher.combine(1)
            }
        }
    }

    let id = UUID()
    var fileName: String
    var keptCopyName: String
    var otherDevice: String
    var kind: Kind
    var at: Date
    var fileURL: URL

    /// "Kept both versions of Budget.xlsx"
    var title: String {
        switch kind {
        case .bothEdited: "Kept both versions of \(fileName)"
        case .editedAndDeleted: "Kept \(fileName), which another device deleted"
        }
    }

    /// Nothing was lost — that is the entire point, so the copy leads with it.
    var body: String {
        switch kind {
        case .bothEdited:
            "Edited on two devices while apart — nothing was lost. "
                + "Look for \(keptCopyName) right beside it."
        case .editedAndDeleted:
            "\(otherDevice) deleted it while this Mac was still editing — nothing was lost. "
                + "Your copy is still there as \(keptCopyName)."
        }
    }

    nonisolated static func == (lhs: Conflict, rhs: Conflict) -> Bool {
        lhs.id == rhs.id && lhs.fileName == rhs.fileName && lhs.keptCopyName == rhs.keptCopyName
            && lhs.otherDevice == rhs.otherDevice && lhs.kind == rhs.kind && lhs.at == rhs.at
            && lhs.fileURL == rhs.fileURL
    }

    nonisolated func hash(into hasher: inout Hasher) {
        hasher.combine(id)
        hasher.combine(fileName)
        hasher.combine(keptCopyName)
        hasher.combine(otherDevice)
        hasher.combine(kind)
        hasher.combine(at)
        hasher.combine(fileURL)
    }
}
