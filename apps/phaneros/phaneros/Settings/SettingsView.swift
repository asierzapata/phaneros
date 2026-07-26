//
//  SettingsView.swift
//  phaneros
//
//  Job #6. Store connection, start at login, notifications, pause — and an advanced
//  corner that exists without being the first thing anyone sees.
//

import ComposableArchitecture
import SwiftUI

struct SettingsView: View {
    let store: StoreOf<SettingsFeature>

    var body: some View {
        TabView {
            GeneralSettings(store: store)
                .tabItem { Label("General", systemImage: "gearshape") }

            NotificationSettings(store: store)
                .tabItem { Label("Notifications", systemImage: "bell") }

            AdvancedSettings(store: store)
                .tabItem { Label("Advanced", systemImage: "wrench.and.screwdriver") }
        }
        .frame(width: 520, height: 380)
    }
}

// MARK: - General

struct GeneralSettings: View {
    let store: StoreOf<SettingsFeature>

    var body: some View {
        SettingsPane {
            SettingRow(title: "Store", subtitle: store.state.storeHost) {
                Button("Change") { /* TODO: wire up first run */ }
                    .buttonStyle(LinkButtonStyle())
            }

            SettingRow(title: "Start Phaneros at login") {
                Toggle(
                    "",
                    isOn: Binding(
                        get: { store.state.startAtLogin },
                        set: { _ in store.send(.toggleStartAtLogin) }
                    )
                )
                .labelsHidden()
                .toggleStyle(.switch)
                .tint(Palette.accent)
            }

            SettingRow(
                title: "Pause all syncing",
                subtitle: store.state.pauseAllSyncing
                    ? "Nothing is moving in either direction." : nil,
                showsDivider: false
            ) {
                Toggle(
                    "",
                    isOn: Binding(
                        get: { store.state.pauseAllSyncing },
                        set: { _ in store.send(.togglePauseAll) }
                    )
                )
                .labelsHidden()
                .toggleStyle(.switch)
                .tint(Palette.accent)
            }
        }
    }
}

// MARK: - Notifications

struct NotificationSettings: View {
    let store: StoreOf<SettingsFeature>

    var body: some View {
        SettingsPane {
            SettingRow(
                title: "Notify me",
                subtitle:
                    "Only for a conflict, a connection that needs reconnecting, and a first sync finishing."
            ) {
                Toggle(
                    "",
                    isOn: Binding(
                        get: { store.state.notificationsEnabled },
                        set: { _ in store.send(.toggleNotifications) }
                    )
                )
                .labelsHidden()
                .toggleStyle(.switch)
                .tint(Palette.accent)
            }

            Text(
                "Phaneros never notifies for routine syncing, brief drops in connection, catching up after sleep, or pausing and resuming. Those show up in the menu bar icon, and wait there until you look."
            )
            .font(.system(size: 12.5))
            .foregroundStyle(Palette.textTertiary)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.top, 16)
        }
    }
}

// MARK: - Advanced

struct AdvancedSettings: View {
    let store: StoreOf<SettingsFeature>

    var body: some View {
        SettingsPane {
            SettingRow(
                title: "Logs", subtitle: "For working out what happened when something went wrong."
            ) {
                Button("Reveal") {
                    store.send(.revealLogsTapped)
                }
                .buttonStyle(LinkButtonStyle())
            }

            SettingRow(
                title: "Reset a drive's sync state",
                subtitle:
                    "Forgets what Phaneros thinks it has already seen and compares everything again. Your files aren't touched.",
                showsDivider: false
            ) {
                Menu("Reset…") {
                    ForEach(store.state.drives) { drive in
                        Button(drive.name) {
                            store.send(.resetDriveSyncTapped(driveID: drive.id))
                        }
                    }
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
            }
        }
    }
}

/// Shared chrome for each settings tab.
private struct SettingsPane<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        ScrollViewIfNeeded {
            VStack(alignment: .leading, spacing: 0) {
                content
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 28)
            .padding(.vertical, 20)
        }
        .background(Palette.card)
    }
}

enum AppPaths {
    static var logs: URL {
        URL.applicationSupportDirectory.appending(path: "phaneros/logs")
    }
}
