//
//  AddDriveSheet.swift
//  phaneros
//
//  Job #5, the easy half: adding is a shorter first run. Same store, same account,
//  so the only questions left are which folder and what to call it.
//

import ComposableArchitecture
import SwiftUI

struct AddDriveSheet: View {
    @Bindable var store: StoreOf<AddDriveFeature>
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        SheetCard(width: 440) {
            Text("Add a drive")
                .font(.system(size: 19, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 8)

            Text("Same store, same account — just point at another folder.")
                .font(.system(size: 13))
                .foregroundStyle(Palette.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 18)

            FolderWell(folder: $store.folder, compact: true)
                .padding(.bottom, 14)

            FieldBox(placeholder: "Drive name", text: $store.name)
                .padding(.bottom, 22)

            HStack(spacing: 10) {
                Spacer()
                Button("Cancel") {
                    store.send(.cancelTapped)
                    dismiss()
                }
                .buttonStyle(QuietButtonStyle())
                .keyboardShortcut(.cancelAction)
                Button("Add drive") {
                    store.send(.submitTapped)
                    dismiss()
                }
                .buttonStyle(PrimaryButtonStyle())
                .keyboardShortcut(.defaultAction)
                .disabled(store.folder == nil || store.name.isEmpty)
            }
        }
        .frame(width: 440)
    }
}
