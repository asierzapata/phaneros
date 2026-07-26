//
//  DateFormatter.swift
//  phaneros
//
//  Date and TimeInterval extensions for phaneros formatting.
//

import Foundation

extension Date {
    /// "just now", "2 min ago", "1 hr ago", "yesterday" — the register the design uses.
    var phanerosRelative: String {
        let seconds = Date.now.timeIntervalSince(self)
        if seconds < 45 { return "just now" }
        if seconds < 90 { return "1 min ago" }
        if seconds < 3600 { return "\(Int(seconds / 60)) min ago" }
        if seconds < 7200 { return "1 hr ago" }
        if seconds < 86_400 { return "\(Int(seconds / 3600)) hr ago" }
        if seconds < 172_800 { return "yesterday" }
        return formatted(.dateTime.month(.abbreviated).day())
    }
}

extension TimeInterval {
    /// "12 minutes", "2 hours" — coarse on purpose; a precise ETA would be a lie.
    var phanerosCoarseDuration: String {
        if self < 90 { return "a minute" }
        if self < 3600 { return "\(Int((self / 60).rounded())) minutes" }
        let hours = Int((self / 3600).rounded())
        return hours == 1 ? "an hour" : "\(hours) hours"
    }
}
