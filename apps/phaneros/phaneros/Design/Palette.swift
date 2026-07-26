//
//  Palette.swift
//  phaneros
//
//  The colour vocabulary from direction 2a/3a. Warm paper neutrals, one terracotta
//  accent, and an amber reserved strictly for "look at me" moments.
//
//  The light values come straight from the design. The dark values are derived:
//  the design pass only specified the menu-bar glyph in dark, so everything else
//  here keeps the same warmth and contrast relationships in reverse.
//

import SwiftUI

extension Color {
    init(light: String, dark: String) {
        self.init(nsColor: NSColor(name: nil) { appearance in
            let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            return NSColor(hex: isDark ? dark : light)
        })
    }

    init(hex: String) {
        self.init(nsColor: NSColor(hex: hex))
    }
}

extension NSColor {
    convenience init(hex: String) {
        var value: UInt64 = 0
        Scanner(string: hex.hasPrefix("#") ? String(hex.dropFirst()) : hex)
            .scanHexInt64(&value)
        self.init(
            srgbRed: CGFloat((value >> 16) & 0xff) / 255,
            green: CGFloat((value >> 8) & 0xff) / 255,
            blue: CGFloat(value & 0xff) / 255,
            alpha: 1
        )
    }
}

/// Every colour the app is allowed to use.
enum Palette {
    // The one accent. Used for the mark at rest, primary buttons, and links.
    static let accent = Color(light: "c9723a", dark: "e79a5b")

    // Reserved for "needs a look". Never red — nothing here is an error.
    static let attention = Color(light: "b8791f", dark: "d9a441")

    // Paused and offline both read as absent rather than wrong.
    static let dormant = Color(light: "a89a80", dark: "8a8072")

    // The only place the app raises its voice: confirming you want to stop a drive.
    static let destructive = Color(light: "8a3f22", dark: "c2603a")

    // Text, warmest to coolest.
    static let textPrimary = Color(light: "1a1a1a", dark: "f2ead9")
    static let textSecondary = Color(light: "3a362e", dark: "d9d2c2")
    static let textTertiary = Color(light: "6b6558", dark: "a8a08e")
    static let textQuaternary = Color(light: "8a8478", dark: "8a8072")
    static let textLabel = Color(light: "9a9488", dark: "7a7364")

    // Small gold eyebrow above section titles in the first-run flow.
    static let eyebrow = Color(light: "a08a63", dark: "c9bda3")

    // Surfaces.
    static let card = Color(light: "ffffff", dark: "221e18")
    static let subtle = Color(light: "f6f5f2", dark: "2a251d")
    static let sunken = Color(light: "ece8de", dark: "332d24")
    static let chip = Color(light: "f0eee6", dark: "2f2921")
    static let border = Color(light: "ece8de", dark: "3a342a")
    static let hairline = Color(light: "000000", dark: "ffffff").opacity(0.08)
    static let dashedBorder = Color(light: "d9c9a8", dark: "4a4234")

    // The near-black used for the device-code panel, which stays dark in both modes.
    static let ink = Color(hex: "1a1712")
    static let inkText = Color(hex: "f2ead9")
    static let inkLabel = Color(hex: "c9bda3")
    static let inkMuted = Color(hex: "8a8072")

    /// Row fill for the selected drive in the sidebar and the glance list.
    static let selection = Color(light: "000000", dark: "ffffff").opacity(0.05)
}

extension Font {
    /// The small uppercase section labels used throughout ("DRIVES", "RECENT ACTIVITY").
    static let sectionLabel = Font.system(size: 11, weight: .semibold)
}

extension View {
    /// Uppercase tracked label, the design's recurring section header.
    func sectionLabelStyle(_ color: Color = Palette.textLabel) -> some View {
        self.font(.sectionLabel)
            .textCase(.uppercase)
            .tracking(0.45)
            .foregroundStyle(color)
    }
}
