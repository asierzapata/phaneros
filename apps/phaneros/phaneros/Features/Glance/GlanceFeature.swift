//
//  GlanceFeature.swift
//  phaneros
//
//  The menu bar popover extracted into its own feature.
//

import ComposableArchitecture
import Foundation

@Reducer
struct GlanceFeature {
    @ObservableState
    struct State: Equatable {
        var drives: [Drive] = []
        var connection: StoreConnection = .notConfigured
        var pauseAllSyncing: Bool = false

        init(
            drives: [Drive] = [],
            connection: StoreConnection = .notConfigured,
            pauseAllSyncing: Bool = false
        ) {
            self.drives = drives
            self.connection = connection
            self.pauseAllSyncing = pauseAllSyncing
        }

        init(from parentState: AppFeature.State) {
            self.drives = parentState.drives
            self.connection = parentState.connection
            self.pauseAllSyncing = parentState.pauseAllSyncing
        }

        var overallMark: MarkState {
            if drives.isEmpty { return .empty }
            if case .unreachable = connection { return .offline }
            if case .tokenRejected = connection { return .attention }
            if drives.contains(where: {
                if case .needsAttention = $0.status { true } else { false }
            }) {
                return .attention
            }
            if drives.contains(where: { $0.status == .offline }) { return .offline }
            if drives.contains(where: { $0.status == .working }) { return .syncing }
            if drives.allSatisfy({ $0.status == .paused }) { return .paused }
            return .upToDate
        }

        var overallSummary: String {
            if drives.isEmpty { return "No drives yet" }
            switch connection {
            case .unreachable(let host, _): return "Can't reach \(host)"
            case .tokenRejected: return "Needs you to reconnect"
            default: break
            }
            switch overallMark {
            case .attention: return "Needs a look"
            case .syncing: return "Syncing…"
            case .paused: return "Paused"
            default: return "Up to date"
            }
        }

        var recentActivity: [ActivityEntry] {
            drives.flatMap(\.activity).sorted { $0.at > $1.at }.prefix(3).map { $0 }
        }

        var allConflicts: [Conflict] {
            drives.flatMap(\.conflicts)
        }
    }

    enum Action: BindableAction {
        case binding(BindingAction<State>)
        case openPhanerosTapped
        case pauseAllTapped
        case resumeAllTapped
        case revealDriveTapped(driveID: Drive.ID)
        case revealConflictTapped(conflict: Conflict)
        case quitTapped
        case delegate(Delegate)

        @CasePathable
        enum Delegate: Equatable {
            case openMainWindow
            case togglePauseAll
            case revealDriveInFinder(driveID: Drive.ID)
            case revealConflictInFinder(conflict: Conflict)
            case quitApp
        }
    }

    var body: some Reducer<State, Action> {
        BindingReducer()
        Reduce { state, action in
            switch action {
            case .binding:
                return .none

            case .openPhanerosTapped:
                return .send(.delegate(.openMainWindow))

            case .pauseAllTapped:
                return .send(.delegate(.togglePauseAll))

            case .resumeAllTapped:
                return .send(.delegate(.togglePauseAll))

            case .revealDriveTapped(let driveID):
                return .send(.delegate(.revealDriveInFinder(driveID: driveID)))

            case .revealConflictTapped(let conflict):
                return .send(.delegate(.revealConflictInFinder(conflict: conflict)))

            case .quitTapped:
                return .send(.delegate(.quitApp))

            case .delegate:
                return .none
            }
        }
    }
}
