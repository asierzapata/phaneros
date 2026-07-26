//
//  SettingsFeature.swift
//  phaneros
//
//  Settings extracted into its own feature.
//

import ComposableArchitecture
import Foundation

@Reducer
struct SettingsFeature {
    @ObservableState
    struct State: Equatable {
        var drives: [Drive] = []
        var connection: StoreConnection = .notConfigured
        var startAtLogin: Bool = true
        var notificationsEnabled: Bool = true
        var pauseAllSyncing: Bool = false

        init(
            drives: [Drive] = [],
            connection: StoreConnection = .notConfigured,
            startAtLogin: Bool = true,
            notificationsEnabled: Bool = true,
            pauseAllSyncing: Bool = false
        ) {
            self.drives = drives
            self.connection = connection
            self.startAtLogin = startAtLogin
            self.notificationsEnabled = notificationsEnabled
            self.pauseAllSyncing = pauseAllSyncing
        }

        init(from parentState: AppFeature.State) {
            self.drives = parentState.drives
            self.connection = parentState.connection
            self.startAtLogin = parentState.startAtLogin
            self.notificationsEnabled = parentState.notificationsEnabled
            self.pauseAllSyncing = parentState.pauseAllSyncing
        }

        var storeHost: String { connection.host ?? "Not connected" }
    }

    enum Action {
        case toggleStartAtLogin
        case toggleNotifications
        case togglePauseAll
        case revealLogsTapped
        case resetDriveSyncTapped(driveID: Drive.ID)
        case delegate(Delegate)

        @CasePathable
        enum Delegate: Equatable {
            case setStartAtLogin(Bool)
            case setNotificationsEnabled(Bool)
            case togglePauseAll
            case revealLogs
            case resetSyncState(driveID: Drive.ID)
        }
    }

    var body: some Reducer<State, Action> {
        Reduce { state, action in
            switch action {
            case .toggleStartAtLogin:
                return .send(.delegate(.setStartAtLogin(!state.startAtLogin)))

            case .toggleNotifications:
                return .send(.delegate(.setNotificationsEnabled(!state.notificationsEnabled)))

            case .togglePauseAll:
                return .send(.delegate(.togglePauseAll))

            case .revealLogsTapped:
                return .send(.delegate(.revealLogs))

            case .resetDriveSyncTapped(let driveID):
                return .send(.delegate(.resetSyncState(driveID: driveID)))

            case .delegate:
                return .none
            }
        }
    }
}
