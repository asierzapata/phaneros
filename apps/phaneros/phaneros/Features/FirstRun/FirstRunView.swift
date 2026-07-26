import ComposableArchitecture
import SwiftUI

struct FirstRunView: View {
    @Bindable var store: StoreOf<FirstRunFeature>
    @Environment(\.dismiss) private var dismiss

    @State private var pastingToken = false
    @State private var expiresAt = Date.now.addingTimeInterval(600)

    private let deviceCode = "FOX-419"

    var body: some View {
        SheetCard(width: 480) {
            switch store.step {
            case .folder: folderStep
            case .store: storeStep
            case .confirm: confirmStep
            }
        }
        .frame(width: 480)
    }

    // MARK: Step 1 — the folder

    private var folderStep: some View {
        VStack(alignment: .leading, spacing: 0) {
            PhanerosMark(state: .upToDate, size: 34)
                .padding(.bottom, 18)

            Text("What should Phaneros keep in sync?")
                .font(.system(size: 21, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 8)

            Text(
                "Pick a folder on this Mac. Phaneros will watch it and match it everywhere else you're signed in."
            )
            .font(.system(size: 13.5))
            .foregroundStyle(Palette.textTertiary)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.bottom, 22)

            FolderWell(folder: $store.folder)
                .padding(.bottom, 18)

            Text("Call this drive")
                .font(.system(size: 12))
                .foregroundStyle(Palette.textQuaternary)
                .padding(.bottom, 6)

            FieldBox(placeholder: "Notes", text: $store.driveName)
                .padding(.bottom, 26)

            HStack(spacing: 10) {
                Spacer()
                Button("Cancel") { dismiss() }
                    .buttonStyle(QuietButtonStyle())
                Button("Continue") { store.send(.nextStepTapped) }
                    .buttonStyle(PrimaryButtonStyle())
                    .disabled(!store.canAdvanceFromFolder)
            }
        }
    }

    // MARK: Step 2 — the store

    private var storeStep: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Connect to a store")
                .font(.system(size: 11))
                .textCase(.uppercase)
                .tracking(0.55)
                .foregroundStyle(Palette.eyebrow)
                .padding(.bottom, 8)

            Text("Where does \"\(store.driveName.isEmpty ? "this drive" : store.driveName)\" live?")
                .font(.system(size: 21, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 8)

            Text(
                "A store is the server your drives sync through — hosted by us, or one you run yourself."
            )
            .font(.system(size: 13.5))
            .foregroundStyle(Palette.textTertiary)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.bottom, 18)

            FieldBox(placeholder: "phaneros.example.com", text: $store.storeHost)
                .padding(.bottom, 24)

            if pastingToken { tokenPanel } else { deviceCodePanel }

            HStack {
                Button(pastingToken ? "Use a device code instead" : "Paste a token instead") {
                    pastingToken.toggle()
                }
                .buttonStyle(.plain)
                .font(.system(size: 12.5, weight: .medium))
                .foregroundStyle(Palette.textQuaternary)

                Spacer()

                Button("Back") { store.send(.previousStepTapped) }
                    .buttonStyle(QuietButtonStyle())

                if pastingToken {
                    Button("Connect") { store.send(.nextStepTapped) }
                        .buttonStyle(PrimaryButtonStyle())
                        .disabled(!store.canAdvanceFromStore)
                }
            }
            .padding(.top, 22)
        }
    }

    private var deviceCodePanel: some View {
        VStack(spacing: 10) {
            Text(
                "On a device that's already connected, open Settings → Connect a device, then enter"
            )
            .font(.system(size: 10.5))
            .textCase(.uppercase)
            .tracking(0.5)
            .foregroundStyle(Palette.inkLabel)
            .multilineTextAlignment(.center)
            .fixedSize(horizontal: false, vertical: true)

            Text(deviceCode)
                .font(.system(size: 34, weight: .semibold, design: .monospaced))
                .tracking(3)
                .foregroundStyle(Palette.inkText)

            TimelineView(.periodic(from: .now, by: 1)) { _ in
                Text("Expires in \(countdown) · waiting for confirmation…")
                    .font(.system(size: 11.5))
                    .foregroundStyle(Palette.inkMuted)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(22)
        .background(Palette.ink, in: .rect(cornerRadius: 12))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "Device code \(deviceCode.map(String.init).joined(separator: " ")). Waiting for confirmation."
        )
    }

    private var tokenPanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Access token")
                .font(.system(size: 12))
                .foregroundStyle(Palette.textQuaternary)
            FieldBox(placeholder: "Paste the token from your store", text: $store.token, monospaced: true)
            Text(
                "You'll find this wherever you set the store up. Phaneros keeps it in your Keychain, not on disk."
            )
            .font(.system(size: 11.5))
            .foregroundStyle(Palette.textQuaternary)
            .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var countdown: String {
        let remaining = max(0, Int(expiresAt.timeIntervalSinceNow))
        return String(format: "%d:%02d", remaining / 60, remaining % 60)
    }

    // MARK: Step 3 — confirm

    private var confirmStep: some View {
        VStack(alignment: .leading, spacing: 0) {
            PhanerosMark(state: .upToDate, size: 34)
                .padding(.bottom, 18)

            Text("Ready to sync \"\(store.driveName)\"")
                .font(.system(size: 21, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 8)

            Text(
                "The first pass copies everything across. After that only changes move, and you shouldn't need to think about this again."
            )
            .font(.system(size: 13.5))
            .foregroundStyle(Palette.textTertiary)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.bottom, 22)

            VStack(spacing: 0) {
                summaryRow("Folder", store.folder?.abbreviatedPath ?? "—")
                summaryRow("Store", store.storeHost.isEmpty ? "—" : store.storeHost, last: true)
            }
            .padding(.horizontal, 14)
            .background(Palette.subtle, in: .rect(cornerRadius: 10))
            .padding(.bottom, 26)

            HStack(spacing: 10) {
                Spacer()
                Button("Back") { store.send(.previousStepTapped) }
                    .buttonStyle(QuietButtonStyle())
                Button("Start syncing") {
                    if let folder = store.folder {
                        store.send(.delegate(.didFinish(name: store.driveName, path: folder)))
                    }
                    dismiss()
                }
                .buttonStyle(PrimaryButtonStyle())
            }
        }
    }

    private func summaryRow(_ label: String, _ value: String, last: Bool = false) -> some View {
        VStack(spacing: 0) {
            HStack {
                Text(label)
                    .font(.system(size: 12.5))
                    .foregroundStyle(Palette.textQuaternary)
                Spacer()
                Text(value)
                    .font(.system(size: 12.5, weight: .medium))
                    .foregroundStyle(Palette.textPrimary)
                    .lineLimit(1)
                    .truncationMode(.head)
            }
            .padding(.vertical, 11)

            if !last {
                Rectangle().fill(Palette.hairline).frame(height: 1)
            }
        }
    }
}
