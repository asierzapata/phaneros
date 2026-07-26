//
//  SettingsFeatureTests.swift
//  phanerosTests
//
//  Tests for the SettingsFeature reducer.
//

import ComposableArchitecture
import Foundation
import Testing

@testable import phaneros

@MainActor
private func makeStore(
    _ state: SettingsFeature.State = SettingsFeature.State()
) -> TestStore<SettingsFeature.State, SettingsFeature.Action> {
    TestStore(initialState: state) {
        SettingsFeature()
    }
}

// MARK: - Initial State

@MainActor
struct SettingsFeatureInitialStateTests {

    @Test func initialState() async {
        let store = await makeStore()
        #expect(store.state.drives.isEmpty)
        #expect(store.state.connection == .notConfigured)
        #expect(store.state.startAtLogin == true)
        #expect(store.state.notificationsEnabled == true)
        #expect(store.state.pauseAllSyncing == false)
        #expect(store.state.storeHost == "Not connected")
    }

    @Test func initialStateFromParent() async {
        let parentState = AppFeature.State()
        let state = SettingsFeature.State(from: parentState)
        #expect(state.startAtLogin == true)
        #expect(state.notificationsEnabled == true)
        #expect(state.pauseAllSyncing == false)
    }
}

// MARK: - Toggle Start At Login

@MainActor
struct SettingsFeatureToggleStartAtLoginTests {

    // The feature owns no state of its own — it reports the flip and the parent applies it.

    @Test func toggleStartAtLoginSendsDelegate() async {
        let store = await makeStore()
        await store.send(.toggleStartAtLogin)
        await store.receive(\.delegate.setStartAtLogin, false)
    }

    @Test func toggleStartAtLoginFromFalseToTrue() async {
        let store = await makeStore(SettingsFeature.State(startAtLogin: false))
        await store.send(.toggleStartAtLogin)
        await store.receive(\.delegate.setStartAtLogin, true)
    }
}

// MARK: - Toggle Notifications

@MainActor
struct SettingsFeatureToggleNotificationsTests {

    @Test func toggleNotificationsSendsDelegate() async {
        let store = await makeStore()
        await store.send(.toggleNotifications)
        await store.receive(\.delegate.setNotificationsEnabled, false)
    }

    @Test func toggleNotificationsFromFalseToTrue() async {
        let store = await makeStore(SettingsFeature.State(notificationsEnabled: false))
        await store.send(.toggleNotifications)
        await store.receive(\.delegate.setNotificationsEnabled, true)
    }
}

// MARK: - Toggle Pause All

@MainActor
struct SettingsFeatureTogglePauseAllTests {

    @Test func togglePauseAllSendsDelegate() async {
        let store = await makeStore()
        await store.send(.togglePauseAll)
        await store.receive(\.delegate.togglePauseAll)
    }
}

// MARK: - Reveal Logs

@MainActor
struct SettingsFeatureRevealLogsTests {

    @Test func revealLogsSendsDelegate() async {
        let store = await makeStore()
        await store.send(.revealLogsTapped)
        await store.receive(\.delegate.revealLogs)
    }
}

// MARK: - Reset Drive Sync

@MainActor
struct SettingsFeatureResetDriveSyncTests {

    @Test func resetDriveSyncSendsDelegate() async {
        let store = await makeStore()
        let driveID = Drive.ID("test-drive-1")

        await store.send(.resetDriveSyncTapped(driveID: driveID))
        await store.receive(\.delegate.resetSyncState, driveID)
    }
}

// MARK: - Derived State

@MainActor
struct SettingsFeatureDerivedStateTests {

    @Test func storeHostFromConnection() {
        let state = SettingsFeature.State(connection: .connected(host: "phaneros.dev"))
        #expect(state.storeHost == "phaneros.dev")
    }

    @Test func storeHostWhenNotConnected() {
        #expect(SettingsFeature.State().storeHost == "Not connected")
    }
}
