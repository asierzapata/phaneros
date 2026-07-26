//
//  GlanceView.swift
//  phaneros
//
//  Job #2: clicking the tray icon. Says what's going on, shows what changed lately,
//  and offers the two or three things anyone actually wants in the moment.
//
//  The design notes that with one drive — the common case — the list collapses away
//  entirely and the header carries the whole story. That's honoured below.
//

import ComposableArchitecture
import SwiftUI

struct GlanceView: View {
    let store: StoreOf<GlanceFeature>
    @Environment(\.openWindow) private var openWindow
    @Environment(\.openSettings) private var openSettings
    @Environment(\.dismiss) private var dismiss

    private var isSingleDrive: Bool { store.drives.count == 1 }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header

            if !store.drives.isEmpty {
                divider
            }

            if !isSingleDrive && !store.drives.isEmpty {
                driveList
                divider
            }

            if let conflict = store.allConflicts.first {
                conflictRow(conflict)
                divider
            }

            if !store.recentActivity.isEmpty {
                lately
                divider
            }

            actions
        }
        .padding(8)
        .frame(width: 340)
    }

    // MARK: Header

    private var header: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 8) {
                PhanerosMark(state: store.overallMark, size: 16)
                Text("Phaneros")
                    .font(.system(size: 14.5, weight: .semibold))
                    .foregroundStyle(Palette.textPrimary)
                Spacer()
                Text(store.overallSummary)
                    .font(.system(size: 12))
                    .foregroundStyle(Palette.textQuaternary)
            }

            if isSingleDrive, let drive = store.drives.first {
                Text("\(drive.name) · \(drive.status.label)")
                    .font(.system(size: 11.5))
                    .foregroundStyle(Palette.textQuaternary)
                    .padding(.leading, 24)
            }
        }
        .padding(.horizontal, 12)
        .padding(.top, 12)
        .padding(.bottom, 10)
        .accessibilityElement(children: .combine)
    }

    // MARK: Drives

    private var driveList: some View {
        VStack(spacing: 0) {
            ForEach(store.drives) { drive in
                Button {
                    store.send(.revealDriveTapped(driveID: drive.id))
                    dismiss()
                } label: {
                    HStack(spacing: 10) {
                        PhanerosMark(state: drive.status.mark, size: 13)
                        VStack(alignment: .leading, spacing: 1) {
                            Text(drive.name)
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(Palette.textPrimary)
                            Text(drive.status.label)
                                .font(.system(size: 11))
                                .foregroundStyle(Palette.textQuaternary)
                        }
                        Spacer()
                        Text("Open ›")
                            .font(.system(size: 11.5, weight: .medium))
                            .foregroundStyle(Palette.textQuaternary)
                    }
                    .padding(.horizontal, 8)
                    .padding(.vertical, 9)
                    .contentShape(.rect(cornerRadius: 8))
                }
                .buttonStyle(GlanceRowStyle())
                .accessibilityLabel("\(drive.name), \(drive.status.shortLabel). Open in Finder")
            }
        }
        .padding(.horizontal, 4)
        .padding(.bottom, 4)
    }

    // MARK: Conflict

    private func conflictRow(_ conflict: Conflict) -> some View {
        Button {
            store.send(.revealConflictTapped(conflict: conflict))
            dismiss()
        } label: {
            HStack(alignment: .top, spacing: 10) {
                PhanerosMark(state: .attention, size: 13)
                    .padding(.top, 1)
                VStack(alignment: .leading, spacing: 2) {
                    Text(conflict.title)
                        .font(.system(size: 12.5, weight: .semibold))
                        .foregroundStyle(Palette.textPrimary)
                    Text("Nothing was lost. Show in Finder ›")
                        .font(.system(size: 11.5))
                        .foregroundStyle(Palette.textQuaternary)
                }
                Spacer(minLength: 0)
            }
            .multilineTextAlignment(.leading)
            .padding(.horizontal, 8)
            .padding(.vertical, 9)
            .contentShape(.rect(cornerRadius: 8))
        }
        .buttonStyle(GlanceRowStyle())
        .padding(.horizontal, 4)
        .padding(.vertical, 4)
    }

    // MARK: Lately

    private var lately: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Lately")
                .font(.system(size: 10, weight: .semibold))
                .textCase(.uppercase)
                .tracking(0.5)
                .foregroundStyle(Palette.eyebrow)

            VStack(alignment: .leading, spacing: 5) {
                ForEach(store.recentActivity) { entry in
                    Text(entry.text)
                        .font(.system(size: 12.5))
                        .foregroundStyle(Palette.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 12)
        .padding(.top, 8)
        .padding(.bottom, 10)
    }

    // MARK: Actions

    private var actions: some View {
        VStack(spacing: 8) {
            HStack(spacing: 6) {
                Button(store.pauseAllSyncing ? "Resume" : "Pause all") {
                    if store.pauseAllSyncing {
                        store.send(.resumeAllTapped)
                    } else {
                        store.send(.pauseAllTapped)
                    }
                    dismiss()
                }
                .buttonStyle(GlanceActionStyle(prominent: false))

                Button("Open Phaneros") {
                    openWindow(id: "main")
                    store.send(.openPhanerosTapped)
                    dismiss()
                }
                .buttonStyle(GlanceActionStyle(prominent: true))
            }

            Button("Quit Phaneros") {
                store.send(.quitTapped)
                dismiss()
            }
            .buttonStyle(QuietButtonStyle(color: Palette.textQuaternary))
            .font(.system(size: 11.5))
        }
        .padding(.horizontal, 4)
        .padding(.top, 8)
        .padding(.bottom, 4)
    }

    private var divider: some View {
        Rectangle()
            .fill(Palette.hairline)
            .frame(height: 1)
            .padding(.horizontal, 4)
    }
}

// MARK: - Styles local to the popover

private struct GlanceRowStyle: ButtonStyle {
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(
                (configuration.isPressed || hovering) ? Palette.selection : .clear,
                in: .rect(cornerRadius: 8)
            )
            .onHover { hovering = $0 }
    }
}

private struct GlanceActionStyle: ButtonStyle {
    var prominent: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12.5, weight: prominent ? .semibold : .medium))
            .foregroundStyle(prominent ? .white : Palette.textPrimary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
            .background(
                (prominent ? Palette.accent : Palette.selection)
                    .opacity(configuration.isPressed ? 0.7 : 1),
                in: .rect(cornerRadius: 7)
            )
            .contentShape(.rect(cornerRadius: 7))
    }
}
