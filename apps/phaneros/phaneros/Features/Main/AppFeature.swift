//
//  AppFeature.swift
//  phaneros
//
//  The root reducer that replaces `AppModel`. Everything the views read lives here,
//  fed by `PhanerosClient` events.
//

import AppKit
import CasePaths
import ComposableArchitecture
import Foundation
import Perception

@Reducer
struct AppFeature {
    let client: PhanerosClient

    /// Set when another device changed a lot while this one was asleep.
    struct CatchUp: Equatable, Hashable {
        var device: String
        var fileCount: Int
    }

    // MARK: - State

    @ObservableState
    struct State: Equatable {
        // Engine state
        var drives: [Drive] = []
        var connection: StoreConnection = .notConfigured
        /// Set when another device changed a lot while this one was asleep.
        var catchUp: CatchUp?

        // UI state
        var selectedDriveID: Drive.ID?
        @Presents var addDrive: AddDriveFeature.State?
        @Presents var firstRun: FirstRunFeature.State?
        @Presents var removeDrive: RemoveDriveFeature.State?

        // Settings
        var startAtLogin = true
        var notificationsEnabled = true
        var pauseAllSyncing = false

        // MARK: Derived

        var selectedDrive: Drive? {
            drives.first { $0.id == selectedDriveID } ?? drives.first
        }

        /// What the menu bar shows. The worst state wins.
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

        /// The single line at the top of the glance.
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

        /// The three most recent things that happened, across all drives.
        var recentActivity: [ActivityEntry] {
            drives.flatMap(\.activity).sorted { $0.at > $1.at }.prefix(3).map { $0 }
        }

        var allConflicts: [Conflict] {
            drives.flatMap(\.conflicts)
        }

        /// The one drive doing a first sync, if any.
        var driveInFirstSync: Drive? {
            drives.first { $0.firstSync != nil }
        }

        var storeHost: String { connection.host ?? "Not connected" }

        var driveDetail: DriveDetailFeature.State? {
            get { selectedDrive.map { DriveDetailFeature.State(drive: $0) } }
            set {
                if let drive = newValue?.drive,
                    let index = drives.firstIndex(where: { $0.id == drive.id })
                {
                    drives[index] = drive
                }
            }
        }

        var glance: GlanceFeature.State {
            get { GlanceFeature.State(from: self) }
            set {
                pauseAllSyncing = newValue.pauseAllSyncing
            }
        }
    }

    // MARK: - Action

    enum Action: BindableAction {
        // Client events
        case drivesChanged([Drive])
        case driveStatus(driveID: String, status: DriveStatus)
        case syncProgress(driveID: String, progress: FirstSyncProgress)
        case firstSyncFinished(driveID: String, fileCount: Int)
        case conflictWritten(driveID: String, conflict: Conflict)
        case activity(driveID: String, entry: ActivityEntry)
        case storeUnreachable(host: String, since: Date)
        case storeReachable(host: String)
        case tokenRejected(host: String)

        // User actions
        case pauseDriveTapped(driveID: String)
        case resumeDriveTapped(driveID: String)
        case addDriveTapped
        case addDriveConfirmed(name: String, path: URL)
        case firstRunTapped
        case removeDriveTapped(drive: Drive)
        case togglePauseAll
        case selectDrive(id: Drive.ID?)
        case dismissCatchUp
        case setCatchUp(device: String, fileCount: Int)
        case startListening

        // Child features
        case addDrive(PresentationAction<AddDriveFeature.Action>)
        case firstRun(PresentationAction<FirstRunFeature.Action>)
        case removeDrive(PresentationAction<RemoveDriveFeature.Action>)
        case glance(GlanceFeature.Action)
        case driveDetail(DriveDetailFeature.Action)
        case settings(SettingsFeature.Action)

        // Bindings (two-way state from views)
        case binding(BindingAction<State>)
    }

    // MARK: - Reducer

    var body: some Reducer<State, Action> {
        BindingReducer()
        // The glance runs its own reducer so its taps become `.delegate` actions the
        // parent can act on. Without this the popover's buttons do nothing at all.
        Scope(state: \.glance, action: \.glance) {
            GlanceFeature()
        }
        Reduce { state, action in
            switch action {
            // MARK: Client Events

            case .drivesChanged(let drives):
                state.drives = drives
                if state.selectedDriveID == nil
                    || !drives.contains(where: { $0.id == state.selectedDriveID })
                {
                    state.selectedDriveID = drives.first?.id
                }
                if drives.isEmpty {
                    state.firstRun = FirstRunFeature.State()
                }
                return .none

            case .driveStatus(let id, let status):
                guard let index = state.drives.firstIndex(where: { $0.id == id }) else {
                    return .none
                }
                state.drives[index].status = status
                return .none

            case .syncProgress(let id, let progress):
                guard let index = state.drives.firstIndex(where: { $0.id == id }) else {
                    return .none
                }
                state.drives[index].firstSync = progress
                return .none

            case .firstSyncFinished(let id, let count):
                guard let index = state.drives.firstIndex(where: { $0.id == id }) else {
                    return .none
                }
                state.drives[index].firstSync = nil
                state.drives[index].status = .upToDate(at: .now)
                if let drive = state.drives.first(where: { $0.id == id }) {
                    Notifier.firstSyncFinished(
                        drive: drive.name,
                        fileCount: count,
                        enabled: state.notificationsEnabled
                    )
                }
                return .none

            case .conflictWritten(let id, let conflict):
                guard let index = state.drives.firstIndex(where: { $0.id == id }) else {
                    return .none
                }
                state.drives[index].conflicts.append(conflict)
                state.drives[index].status = .needsAttention(reason: "conflict")
                Notifier.conflict(conflict, enabled: state.notificationsEnabled)
                return .none

            case .activity(let id, let entry):
                guard let index = state.drives.firstIndex(where: { $0.id == id }) else {
                    return .none
                }
                state.drives[index].activity.insert(entry, at: 0)
                return .none

            case .storeUnreachable(let host, let since):
                state.connection = .unreachable(host: host, since: since)
                return .none

            case .storeReachable(let host):
                state.connection = .connected(host: host)
                return .none

            case .tokenRejected(let host):
                state.connection = .tokenRejected(host: host)
                Notifier.tokenRejected(host: host, enabled: state.notificationsEnabled)
                return .none

            // MARK: User Actions

            case .pauseDriveTapped(let id):
                return .run { _ in await client.send(.pause(driveID: id)) }

            case .resumeDriveTapped(let id):
                return .run { _ in await client.send(.resume(driveID: id)) }

            case .addDriveTapped:
                state.addDrive = AddDriveFeature.State()
                return .none

            case .addDriveConfirmed(let name, let path):
                return .run { [host = state.connection.host ?? ""] _ in
                    await client.send(
                        .addDrive(
                            name: name,
                            path: path,
                            storeURL: host,
                            driveID: UUID().uuidString
                        )
                    )
                }

            case .firstRunTapped:
                state.firstRun = FirstRunFeature.State()
                return .none

            case .removeDriveTapped(let drive):
                state.removeDrive = RemoveDriveFeature.State(drive: drive)
                return .none

            case .togglePauseAll:
                state.pauseAllSyncing.toggle()
                return .run { [pauseAll = state.pauseAllSyncing] _ in
                    await client.send(pauseAll ? .pauseAll : .resumeAll)
                }

            case .selectDrive(let id):
                state.selectedDriveID = id
                return .none

            case .dismissCatchUp:
                state.catchUp = nil
                return .none

            case .setCatchUp(let device, let fileCount):
                state.catchUp = CatchUp(device: device, fileCount: fileCount)
                return .none

            case .startListening:
                return .run { send in
                    for await event in await client.events() {
                        await send(mapEvent(event))
                    }
                }

            case .addDrive(.presented(.delegate(.didFinish(let name, let path)))):
                state.addDrive = nil
                return .run { [host = state.connection.host ?? ""] _ in
                    await client.send(
                        .addDrive(
                            name: name,
                            path: path,
                            storeURL: host,
                            driveID: UUID().uuidString
                        )
                    )
                }

            case .addDrive(.dismiss):
                state.addDrive = nil
                return .none

            case .addDrive:
                return .none

            case .firstRun(.presented(.delegate(.didFinish(let name, let path)))):
                state.firstRun = nil
                return .run { [host = state.connection.host ?? ""] _ in
                    await client.send(
                        .addDrive(
                            name: name,
                            path: path,
                            storeURL: host,
                            driveID: UUID().uuidString
                        )
                    )
                }

            case .firstRun(.dismiss):
                state.firstRun = nil
                return .none

            case .firstRun:
                return .none

            case .removeDrive(.presented(.delegate(.didConfirmRemoval(let driveID)))):
                state.removeDrive = nil
                return .run { _ in await client.send(.removeDrive(driveID: driveID)) }

            case .removeDrive(.dismiss):
                state.removeDrive = nil
                return .none

            case .removeDrive:
                return .none

            case .glance(.delegate(.openMainWindow)):
                return .run { _ in
                    await MainActor.run {
                        NSApp.setActivationPolicy(.regular)
                        NSApp.activate(ignoringOtherApps: true)
                        for window in NSApp.windows {
                            if window.title == "Phaneros" {
                                window.makeKeyAndOrderFront(nil)
                                break
                            }
                        }
                    }
                }

            case .glance(.delegate(.togglePauseAll)):
                return .send(.togglePauseAll)

            case .glance(.delegate(.revealDriveInFinder(let driveID))):
                guard let drive = state.drives.first(where: { $0.id == driveID }) else {
                    return .none
                }
                return .run { _ in
                    await MainActor.run {
                        NSWorkspace.shared.activateFileViewerSelecting([drive.path])
                    }
                }

            case .glance(.delegate(.revealConflictInFinder(let conflict))):
                return .run { _ in
                    await MainActor.run {
                        NSWorkspace.shared.activateFileViewerSelecting([conflict.fileURL])
                    }
                }

            case .glance(.delegate(.quitApp)):
                return .run { _ in
                    await MainActor.run {
                        NSApp.terminate(nil)
                    }
                }

            case .glance:
                return .none

            case .driveDetail(.pauseTapped):
                guard let drive = state.selectedDrive else { return .none }
                return .run { _ in await client.send(.pause(driveID: drive.id)) }

            case .driveDetail(.resumeTapped):
                guard let drive = state.selectedDrive else { return .none }
                return .run { _ in await client.send(.resume(driveID: drive.id)) }

            case .driveDetail(.revealInFinder):
                guard let drive = state.selectedDrive else { return .none }
                return .run { _ in
                    await MainActor.run {
                        NSWorkspace.shared.activateFileViewerSelecting([drive.path])
                    }
                }

            case .driveDetail(.revealConflict(let conflictID)):
                guard let drive = state.selectedDrive,
                    let conflict = drive.conflicts.first(where: { $0.id == conflictID })
                else { return .none }
                return .run { _ in
                    await MainActor.run {
                        NSWorkspace.shared.activateFileViewerSelecting([conflict.fileURL])
                    }
                }

            case .driveDetail(.delegate(.pauseDrive(let driveID))):
                return .run { _ in await client.send(.pause(driveID: driveID)) }

            case .driveDetail(.delegate(.resumeDrive(let driveID))):
                return .run { _ in await client.send(.resume(driveID: driveID)) }

            case .driveDetail(.delegate(.didRevealInFinder)):
                return .none

            case .driveDetail(.delegate(.didRevealConflict)):
                return .none

            case .driveDetail:
                return .none

            case .settings(.delegate(.setStartAtLogin(let value))):
                state.startAtLogin = value
                return .none

            case .settings(.delegate(.setNotificationsEnabled(let value))):
                state.notificationsEnabled = value
                return .none

            case .settings(.delegate(.togglePauseAll)):
                return .send(.togglePauseAll)

            case .settings(.delegate(.revealLogs)):
                return .run { _ in
                    await MainActor.run {
                        NSWorkspace.shared.activateFileViewerSelecting([AppPaths.logs])
                    }
                }

            case .settings(.delegate(.resetSyncState(let driveID))):
                return .run { _ in await client.send(.resetSyncState(driveID: driveID)) }

            case .settings:
                return .none

            case .binding:
                return .none
            }
        }
        .ifLet(\.$addDrive, action: \.addDrive) {
            AddDriveFeature()
        }
        .ifLet(\.$firstRun, action: \.firstRun) {
            FirstRunFeature()
        }
        .ifLet(\.$removeDrive, action: \.removeDrive) {
            RemoveDriveFeature()
        }
    }

    // MARK: - Event Mapping

    private func mapEvent(_ event: Event) -> Action {
        switch event {
        case .drivesChanged(let drives):
            return .drivesChanged(drives)
        case .driveStatus(let id, let status):
            return .driveStatus(driveID: id, status: status)
        case .syncProgress(let id, let progress):
            return .syncProgress(driveID: id, progress: progress)
        case .firstSyncFinished(let id, let count):
            return .firstSyncFinished(driveID: id, fileCount: count)
        case .conflictWritten(let id, let conflict):
            return .conflictWritten(driveID: id, conflict: conflict)
        case .activity(let id, let entry):
            return .activity(driveID: id, entry: entry)
        case .storeUnreachable(let host, let since):
            return .storeUnreachable(host: host, since: since)
        case .storeReachable(let host):
            return .storeReachable(host: host)
        case .tokenRejected(let host):
            return .tokenRejected(host: host)
        }
    }
}
