//
//  Components.swift
//  phaneros
//
//  The handful of controls the design reuses. Built as ButtonStyles rather than
//  tapped-on-tap-gesture rectangles so focus rings, keyboard activation and
//  VoiceOver come for free — the brief treats those as assumed, not optional.
//

import SwiftUI

// MARK: - Snapshotting

extension EnvironmentValues {
    /// True while the snapshot harness is rendering.
    ///
    /// `ImageRenderer` rasterizes a `ScrollView` as empty, so scrolling containers
    /// unwrap themselves during a snapshot. Nothing else in the app reads this.
    @Entry var isSnapshotting = false
}

/// A `ScrollView` everywhere except inside the snapshot harness.
struct ScrollViewIfNeeded<Content: View>: View {
    @Environment(\.isSnapshotting) private var isSnapshotting
    @ViewBuilder var content: Content

    var body: some View {
        if isSnapshotting {
            content
        } else {
            ScrollView { content }
        }
    }
}

// MARK: - Buttons

/// Terracotta fill, white label. One per screen, on the thing you most likely want.
struct PrimaryButtonStyle: ButtonStyle {
    var fill: Color = Palette.accent
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .semibold))
            .foregroundStyle(.white)
            .padding(.horizontal, 20)
            .padding(.vertical, 9)
            .background(fill.opacity(configuration.isPressed ? 0.82 : 1), in: .rect(cornerRadius: 8))
            .contentShape(.rect(cornerRadius: 8))
            // A primary action you can't take yet has to look like it.
            .opacity(isEnabled ? 1 : 0.4)
    }
}

/// Warm neutral chip. Everything that isn't the primary action.
struct SecondaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(Palette.textSecondary)
            .padding(.horizontal, 14)
            .padding(.vertical, 9)
            .background(
                Palette.chip.opacity(configuration.isPressed ? 0.6 : 1),
                in: .rect(cornerRadius: 8)
            )
            .contentShape(.rect(cornerRadius: 8))
    }
}

/// Text-only. Cancel, Back, and the like.
struct QuietButtonStyle: ButtonStyle {
    var color: Color = Palette.textTertiary

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(color.opacity(configuration.isPressed ? 0.6 : 1))
            .padding(.horizontal, 16)
            .padding(.vertical, 9)
            .contentShape(.rect)
    }
}

/// An inline accent link — "Change", "Reconnect", "Show in Finder".
struct LinkButtonStyle: ButtonStyle {
    var size: CGFloat = 13

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: size, weight: .semibold))
            .foregroundStyle(Palette.accent.opacity(configuration.isPressed ? 0.6 : 1))
            .contentShape(.rect)
    }
}

// MARK: - Fields

/// The warm inset box used for the drive name and store address.
struct FieldBox: View {
    var placeholder: String
    @Binding var text: String
    var monospaced = false

    var body: some View {
        TextField(placeholder, text: $text)
            .textFieldStyle(.plain)
            .font(.system(size: 14, weight: .medium, design: monospaced ? .monospaced : .default))
            .foregroundStyle(Palette.textPrimary)
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(Palette.subtle, in: .rect(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(Palette.border, lineWidth: 1)
            )
    }
}

/// The dashed folder well. Opens a real `NSOpenPanel`; once a folder is picked it
/// shows the path instead of the invitation.
struct FolderWell: View {
    @Binding var folder: URL?
    var compact = false

    var body: some View {
        Button {
            pick()
        } label: {
            VStack(spacing: 4) {
                if let folder {
                    Text(folder.lastPathComponent)
                        .font(.system(size: 13.5, weight: .semibold))
                        .foregroundStyle(Palette.textPrimary)
                    Text(folder.abbreviatedPath)
                        .font(.system(size: 11.5))
                        .foregroundStyle(Palette.textQuaternary)
                } else {
                    Text("Choose a folder…")
                        .font(.system(size: 13.5, weight: .medium))
                        .foregroundStyle(Palette.textTertiary)
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, compact ? 18 : 22)
            .contentShape(.rect(cornerRadius: 10))
        }
        .buttonStyle(.plain)
        .background(
            RoundedRectangle(cornerRadius: 10)
                .strokeBorder(
                    Palette.dashedBorder,
                    style: StrokeStyle(lineWidth: 1.5, dash: [5, 4])
                )
        )
        .accessibilityLabel(folder.map { "Folder: \($0.lastPathComponent). Change" } ?? "Choose a folder")
    }

    private func pick() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Choose"
        panel.message = "Pick a folder for Phaneros to keep in sync."
        if panel.runModal() == .OK { folder = panel.url }
    }
}

extension URL {
    var abbreviatedPath: String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        let full = path(percentEncoded: false)
        return full.hasPrefix(home) ? "~" + full.dropFirst(home.count) : full
    }
}

// MARK: - Settings rows

/// A labelled row with a hairline under it, as used throughout Settings.
struct SettingRow<Trailing: View>: View {
    var title: String
    var subtitle: String?
    var showsDivider = true
    @ViewBuilder var trailing: Trailing

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.system(size: 14.5, weight: .medium))
                        .foregroundStyle(Palette.textPrimary)
                    if let subtitle {
                        Text(subtitle)
                            .font(.system(size: 12.5))
                            .foregroundStyle(Palette.textQuaternary)
                    }
                }
                Spacer()
                trailing
            }
            .padding(.vertical, 14)

            if showsDivider {
                Rectangle()
                    .fill(Palette.hairline)
                    .frame(height: 1)
            }
        }
    }
}

// MARK: - Cards

/// The soft warm card used for the "when things aren't fine" tiles.
struct SoftCard<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        content
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Palette.subtle, in: .rect(cornerRadius: 10))
    }
}

/// A modal card: white, generously padded, heavily shadowed. Used by every sheet.
struct SheetCard<Content: View>: View {
    var width: CGFloat = 440
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            content
        }
        .padding(.horizontal, 36)
        .padding(.vertical, 32)
        .frame(width: width, alignment: .leading)
        .background(Palette.card)
    }
}
