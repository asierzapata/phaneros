//
//  RemoveDriveSheet.swift
//  phaneros
//
//  Job #5, the sharp edge. Stopping a drive must not read as "delete my files",
//  because it isn't — so the reassurance is the body copy, not a footnote, and it
//  says what *does* happen rather than only what doesn't.
//
//  Cancel is the default action. The destructive button is deliberately the darker
//  burnt tone rather than the accent: it's the one moment the app is allowed to look
//  serious, and it should not be mistaken for the friendly primary everywhere else.
//

import ComposableArchitecture
import SwiftUI

struct RemoveDriveSheet: View {
    @Bindable var store: StoreOf<RemoveDriveFeature>
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        guard let drive = store.state.drive else { return AnyView(EmptyView()) }
        return AnyView(
            SheetCard(width: 440) {
                Text("Stop syncing \"\(drive.name)\"?")
                    .font(.system(size: 19, weight: .semibold))
                    .foregroundStyle(Palette.textPrimary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.bottom, 10)

                Text(
                    "This does not delete anything. The folder and every file in it stay on this Mac exactly as they are — Phaneros just stops watching it and stops sending changes to other devices."
                )
                .font(.system(size: 13.5))
                .foregroundStyle(Palette.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 22)

                HStack(spacing: 10) {
                    Spacer()
                    Button("Cancel") {
                        store.send(.cancelTapped)
                        dismiss()
                    }
                    .buttonStyle(QuietButtonStyle())
                    .keyboardShortcut(.defaultAction)

                    Button("Stop syncing") {
                        store.send(.confirmTapped)
                        dismiss()
                    }
                    .buttonStyle(PrimaryButtonStyle(fill: Palette.destructive))
                }
            }
            .frame(width: 440)
        )
    }
}
