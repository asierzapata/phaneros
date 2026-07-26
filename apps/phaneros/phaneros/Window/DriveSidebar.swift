//
//  DriveSidebar.swift
//  phaneros
//
//  Each row carries its own status, so the list answers "is everything fine?"
//  without anyone having to click into a drive.
//

import ComposableArchitecture
import SwiftUI

struct DriveSidebar: View {
    @Bindable var store: StoreOf<AppFeature>

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Drives")
                .sectionLabelStyle()
                .padding(.horizontal, 10)
                .padding(.top, 6)
                .padding(.bottom, 12)

            ForEach(store.state.drives) { drive in
                DriveRow(drive: drive, isSelected: drive.id == store.state.selectedDrive?.id) {
                    store.send(.selectDrive(id: drive.id))
                }
                .contextMenu {
                    Button(drive.status == .paused ? "Resume" : "Pause") {
                        if drive.status == .paused {
                            store.send(.resumeDriveTapped(driveID: drive.id))
                        } else {
                            store.send(.pauseDriveTapped(driveID: drive.id))
                        }
                    }
                    Button("Open in Finder") {
                        NSWorkspace.shared.activateFileViewerSelecting([drive.path])
                    }
                    Divider()
                    Button("Stop syncing…") { store.send(.removeDriveTapped(drive: drive)) }
                }
            }

            Rectangle()
                .fill(Palette.hairline)
                .frame(height: 1)
                .padding(.horizontal, 10)
                .padding(.vertical, 14)

            Button("+ Add a drive") { store.send(.addDriveTapped) }
                .buttonStyle(LinkButtonStyle())
                .padding(.horizontal, 10)
                .padding(.vertical, 4)

            Spacer()
        }
        .padding(.horizontal, 14)
        .padding(.top, 14)
    }
}

private struct DriveRow: View {
    var drive: Drive
    var isSelected: Bool
    var select: () -> Void

    @State private var hovering = false

    var body: some View {
        Button(action: select) {
            HStack(spacing: 10) {
                PhanerosMark(state: drive.status.mark, size: 16)
                VStack(alignment: .leading, spacing: 1) {
                    Text(drive.name)
                        .font(.system(size: 14, weight: isSelected ? .semibold : .medium))
                        .foregroundStyle(isSelected ? Palette.textPrimary : Palette.textSecondary)
                    Text(drive.status.shortLabel)
                        .font(.system(size: 11.5))
                        .foregroundStyle(Palette.textQuaternary)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 11)
            .background(
                isSelected
                    ? Palette.selection : (hovering ? Palette.selection.opacity(0.5) : .clear),
                in: .rect(cornerRadius: 9)
            )
            .contentShape(.rect(cornerRadius: 9))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .padding(.bottom, 4)
        .accessibilityLabel("\(drive.name), \(drive.status.shortLabel)")
        .accessibilityAddTraits(isSelected ? [.isSelected] : [])
    }
}
