//
//  AppFeatureTests.swift
//  phanerosTests
//
//  Tests for the root reducer that replaces AppModel.
//  Written BEFORE the feature exists — they define the expected behavior.
//

import CasePaths
import ComposableArchitecture
import Foundation
import Perception
import Testing

@testable import phaneros

@MainActor
private func makeStore(
    client: MockClient = MockClient()
) -> TestStore<AppFeature.State, AppFeature.Action> {
    let store = TestStore(initialState: AppFeature.State()) {
        AppFeature(client: client)
    } withDependencies: {
        $0.continuousClock = ImmediateClock()
    }
    // The tests assert state via read-only `#expect`s inside `send`'s trailing
    // closure rather than by mutating the closure's `inout State`, so we relax
    // TCA's default strict exhaustivity (which requires the closure to drive
    // every observed change).
    store.exhaustivity = .off
    return store
}

@MainActor
private func drive(
    _ id: String,
    name: String = "Test",
    status: DriveStatus = .upToDate(at: .now)
) -> Drive {
    Drive(id: id, name: name, path: URL(fileURLWithPath: "/tmp/\(name)"), status: status)
}

// MARK: - Initial State

@MainActor
struct AppFeatureInitialStateTests {

    @Test func initialState() async {
        let store = await makeStore()
        await store.send(.selectDrive(id: nil)) { state in
            #expect(state.drives.isEmpty)
            #expect(state.connection == .notConfigured)
            #expect(state.selectedDriveID == nil)
            #expect(state.addDrive == nil)
            #expect(state.removeDrive == nil)
            #expect(state.firstRun == nil)
            #expect(state.catchUp == nil)
            #expect(state.startAtLogin == true)
            #expect(state.notificationsEnabled == true)
            #expect(state.pauseAllSyncing == false)
        }
    }
}

// MARK: - Drives Changed

@MainActor
struct AppFeatureDrivesChangedTests {

    @Test func drivesChangedSetsDrives() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes"), drive("b", name: "Work")]

        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
    }

    @Test func drivesChangedEmptySetsFirstRun() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes")]

        // First set some drives so firstRun can be tested
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        // Now send empty drives
        await store.send(.drivesChanged([])) { state in
            state.drives = []
            state.firstRun = FirstRunFeature.State()
        }
    }

    @Test func drivesChangedPreservesSelection() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes"), drive("b", name: "Work")]

        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        // Select drive "b", then send drives changed — selection should be kept
        await store.send(.selectDrive(id: "b")) { state in
            state.selectedDriveID = "b"
        }

        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            // selectedDriveID stays "b" since it's still valid
        }
    }
}

// MARK: - Drive Status

@MainActor
struct AppFeatureDriveStatusTests {

    @Test func driveStatusUpdatesDrive() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes"), drive("b", name: "Work")]

        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        await store.send(.driveStatus(driveID: "a", status: .paused)) { state in
            state.drives[0].status = .paused
        }
    }
}

// MARK: - Sync Progress

@MainActor
struct AppFeatureSyncProgressTests {

    @Test func syncProgressSetsFirstSync() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes")]

        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        let progress = FirstSyncProgress(filesDone: 3000, filesTotal: 10000)
        await store.send(.syncProgress(driveID: "a", progress: progress)) { state in
            state.drives[0].firstSync = progress
        }
    }
}

// MARK: - First Sync Finished

@MainActor
struct AppFeatureFirstSyncFinishedTests {

    @Test func firstSyncFinishedClearsProgressAndSetsUpToDate() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes")]

        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        let progress = FirstSyncProgress(filesDone: 3000, filesTotal: 10000)
        await store.send(.syncProgress(driveID: "a", progress: progress)) { state in
            state.drives[0].firstSync = progress
        }

        await store.send(.firstSyncFinished(driveID: "a", fileCount: 3000)) { state in
            // The reducer stamps `.upToDate(at: .now)` itself; that instant
            // can't be predicted, so compare the date-less `mark` instead of
            // re-stamping `.now` in the closure (which would diverge under
            // `exhaustivity = .off`, where the closure sees post-reducer state).
            #expect(state.drives[0].firstSync == nil)
            #expect(state.drives[0].status.mark == .upToDate)
        }
    }
}

// MARK: - Conflict Written

@MainActor
struct AppFeatureConflictWrittenTests {

    @Test func conflictWrittenAddsConflictAndSetsNeedsAttention() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Work")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        let conflict = Conflict(
            fileName: "Budget.xlsx",
            keptCopyName: "Budget (Kept Copy).xlsx",
            otherDevice: "Mac mini",
            kind: .bothEdited,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/Budget.xlsx")
        )

        await store.send(.conflictWritten(driveID: "a", conflict: conflict)) { state in
            // Under `exhaustivity = .off` the assert closure is handed the
            // reducer's already-updated state, so we verify via read-only
            // `#expect`s rather than re-applying the reducer's mutations
            // (which would double-append the conflict and diverge).
            #expect(state.drives[0].conflicts == [conflict])
            #expect(state.drives[0].status == .needsAttention(reason: "conflict"))
        }
    }
}

// MARK: - Store Connection

@MainActor
struct AppFeatureConnectionTests {

    @Test func storeUnreachable() async {
        let store = await makeStore()
        let since = Date()

        await store.send(.storeUnreachable(host: "phaneros.dev", since: since)) { state in
            state.connection = .unreachable(host: "phaneros.dev", since: since)
        }
    }

    @Test func storeReachable() async {
        let store = await makeStore()

        // Capture `since` once so the action argument and the expected-state
        // mutation refer to the same instant (a second `.now` would differ and
        // diverge under `exhaustivity = .off`).
        let since = Date()
        await store.send(.storeUnreachable(host: "phaneros.dev", since: since)) { state in
            state.connection = .unreachable(host: "phaneros.dev", since: since)
        }

        await store.send(.storeReachable(host: "phaneros.dev")) { state in
            state.connection = .connected(host: "phaneros.dev")
        }
    }

    @Test func tokenRejected() async {
        let store = await makeStore()

        await store.send(.tokenRejected(host: "phaneros.dev")) { state in
            state.connection = .tokenRejected(host: "phaneros.dev")
        }
    }
}

// MARK: - Commands

@MainActor
struct AppFeatureCommandTests {

    @Test func pauseDriveSendsCommand() async {
        let client = MockClient()
        let store = await makeStore(client: client)
        let drives = [drive("a", name: "Notes")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        await store.send(.pauseDriveTapped(driveID: "a"))
        #expect(client.commandsReceived.contains(.pause(driveID: "a")))
    }

    @Test func resumeDriveSendsCommand() async {
        let client = MockClient()
        let store = await makeStore(client: client)
        let drives = [drive("a", name: "Notes", status: .paused)]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        await store.send(.resumeDriveTapped(driveID: "a"))
        #expect(client.commandsReceived.contains(.resume(driveID: "a")))
    }

    @Test func addDriveSendsCommand() async {
        let client = MockClient()
        let store = await makeStore(client: client)

        await store.send(.addDriveTapped) { state in
            state.addDrive = AddDriveFeature.State()
        }

        let path = URL(fileURLWithPath: "/tmp/NewDrive")
        await store.send(.addDrive(.presented(.delegate(.didFinish(name: "New Drive", path: path))))) { state in
            state.addDrive = nil
        }
        #expect(client.commandsReceived.count == 1)
        if case .addDrive = client.commandsReceived.first {
            // expected
        } else {
            Issue.record("Expected addDrive command")
        }
    }

    @Test func removeDriveSendsCommand() async {
        let client = MockClient()
        let store = await makeStore(client: client)
        let drives = [drive("a", name: "Notes")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        await store.send(.removeDriveTapped(drive: drives[0])) { state in
            state.removeDrive = RemoveDriveFeature.State(drive: drives[0])
        }

        await store.send(
            .removeDrive(.presented(.delegate(.didConfirmRemoval(driveID: "a"))))
        ) { state in
            state.removeDrive = nil
        }
        #expect(client.commandsReceived.first == .removeDrive(driveID: "a"))
    }

    @Test func togglePauseAllPauseAll() async {
        let client = MockClient()
        let store = await makeStore(client: client)

        await store.send(.togglePauseAll) { state in
            state.pauseAllSyncing = true
        }
        #expect(client.commandsReceived.first == .pauseAll)
    }

    @Test func togglePauseAllResumeAll() async {
        let client = MockClient()
        let store = await makeStore(client: client)

        // First pause
        await store.send(.togglePauseAll) { state in
            state.pauseAllSyncing = true
        }

        // Then resume
        await store.send(.togglePauseAll) { state in
            state.pauseAllSyncing = false
        }
        #expect(client.commandsReceived.last == .resumeAll)
    }
}

// MARK: - UI Actions

@MainActor
struct AppFeatureUIActionTests {

    @Test func selectDrive() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes"), drive("b", name: "Work")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        await store.send(.selectDrive(id: "b")) { state in
            state.selectedDriveID = "b"
        }
    }

    @Test func dismissCatchUp() async {
        let store = await makeStore()
        await store.send(.setCatchUp(device: "Mac mini", fileCount: 42)) { state in
            state.catchUp = AppFeature.CatchUp(device: "Mac mini", fileCount: 42)
        }

        await store.send(.dismissCatchUp) { state in
            state.catchUp = nil
        }
    }
}

// MARK: - Computed Properties

@MainActor
struct AppFeatureComputedPropertyTests {

    @Test func selectedDriveReturnsCorrectDrive() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes"), drive("b", name: "Work")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.selectedDrive?.id == "a")
        }

        await store.send(.selectDrive(id: "b")) { state in
            state.selectedDriveID = "b"
            #expect(state.selectedDrive?.id == "b")
        }
    }

    @Test func selectedDriveFallsToFirstWhenNil() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes"), drive("b", name: "Work")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
        await store.send(.selectDrive(id: nil)) { state in
            state.selectedDriveID = nil
            #expect(state.selectedDrive?.id == "a")
        }
    }

    @Test func overallMarkEmpty() async {
        let store = await makeStore()
        await store.send(.selectDrive(id: nil)) { state in
            #expect(state.overallMark == .empty)
        }
    }

    @Test func overallMarkOffline() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
        let since = Date()
        await store.send(.storeUnreachable(host: "phaneros.dev", since: since)) { state in
            state.connection = .unreachable(host: "phaneros.dev", since: since)
            #expect(state.overallMark == .offline)
        }
    }

    @Test func overallMarkAttentionOnTokenRejected() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
        await store.send(.tokenRejected(host: "phaneros.dev")) { state in
            state.connection = .tokenRejected(host: "phaneros.dev")
            #expect(state.overallMark == .attention)
        }
    }

    @Test func overallMarkAttentionOnConflict() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Work")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
        let conflict = Conflict(
            fileName: "Budget.xlsx",
            keptCopyName: "Budget (Kept Copy).xlsx",
            otherDevice: "Mac mini",
            kind: .bothEdited,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/Budget.xlsx")
        )
        await store.send(.conflictWritten(driveID: "a", conflict: conflict)) { state in
            #expect(state.drives[0].conflicts == [conflict])
            #expect(state.drives[0].status == .needsAttention(reason: "conflict"))
            #expect(state.overallMark == .attention)
        }
    }

    @Test func overallMarkOfflineOnOfflineDrive() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes", status: .offline)]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.overallMark == .offline)
        }
    }

    @Test func overallMarkSyncing() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes", status: .working)]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.overallMark == .syncing)
        }
    }

    @Test func overallMarkPaused() async {
        let store = await makeStore()
        let drives = [
            drive("a", name: "Notes", status: .paused),
            drive("b", name: "Work", status: .paused),
        ]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.overallMark == .paused)
        }
    }

    @Test func overallMarkUpToDate() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes", status: .upToDate(at: .now))]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.overallMark == .upToDate)
        }
    }

    @Test func overallSummaryEmpty() async {
        let store = await makeStore()
        await store.send(.selectDrive(id: nil)) { state in
            #expect(state.overallSummary == "No drives yet")
        }
    }

    @Test func overallSummaryUnreachable() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
        let since = Date()
        await store.send(.storeUnreachable(host: "phaneros.dev", since: since)) { state in
            state.connection = .unreachable(host: "phaneros.dev", since: since)
            #expect(state.overallSummary == "Can't reach phaneros.dev")
        }
    }

    @Test func overallSummaryTokenRejected() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
        await store.send(.tokenRejected(host: "phaneros.dev")) { state in
            state.connection = .tokenRejected(host: "phaneros.dev")
            #expect(state.overallSummary == "Needs you to reconnect")
        }
    }

    @Test func overallSummaryAttention() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Work")]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
        let conflict = Conflict(
            fileName: "Budget.xlsx",
            keptCopyName: "Budget (Kept Copy).xlsx",
            otherDevice: "Mac mini",
            kind: .bothEdited,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/Budget.xlsx")
        )
        await store.send(.conflictWritten(driveID: "a", conflict: conflict)) { state in
            #expect(state.drives[0].conflicts == [conflict])
            #expect(state.drives[0].status == .needsAttention(reason: "conflict"))
            #expect(state.overallSummary == "Needs a look")
        }
    }

    @Test func overallSummarySyncing() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes", status: .working)]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.overallSummary == "Syncing…")
        }
    }

    @Test func overallSummaryPaused() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes", status: .paused)]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.overallSummary == "Paused")
        }
    }

    @Test func overallSummaryUpToDate() async {
        let store = await makeStore()
        let drives = [drive("a", name: "Notes", status: .upToDate(at: .now))]
        await store.send(.drivesChanged(drives)) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
            #expect(state.overallSummary == "Up to date")
        }
    }

    @Test func recentActivityReturnsTopThree() async {
        let store = await makeStore()
        let a1 = ActivityEntry(text: "First", at: Date(timeIntervalSinceNow: -100))
        let a2 = ActivityEntry(text: "Second", at: Date(timeIntervalSinceNow: -50))
        let a3 = ActivityEntry(text: "Third", at: Date(timeIntervalSinceNow: -200))
        let a4 = ActivityEntry(text: "Fourth", at: Date(timeIntervalSinceNow: -300))

        let d1 = Drive(
            id: "a", name: "Notes",
            path: URL(fileURLWithPath: "/tmp/Notes"),
            status: .upToDate(at: .now),
            activity: [a1, a3]
        )
        let d2 = Drive(
            id: "b", name: "Work",
            path: URL(fileURLWithPath: "/tmp/Work"),
            status: .upToDate(at: .now),
            activity: [a2, a4]
        )

        await store.send(.drivesChanged([d1, d2])) { state in
            state.drives = [d1, d2]
            state.selectedDriveID = "a"
            state.firstRun = nil

            let recent = state.recentActivity
            #expect(recent.count == 3)
            #expect(recent[0].text == "Second")
            #expect(recent[1].text == "First")
            #expect(recent[2].text == "Third")
        }
    }

    @Test func recentActivityEmpty() async {
        let store = await makeStore()
        await store.send(.selectDrive(id: nil)) { state in
            #expect(state.recentActivity.isEmpty)
        }
    }
}

// MARK: - Event Listener

@MainActor
struct AppFeatureEventListenerTests {

    @Test func eventsReceivedFromClient() async {
        let client = MockClient()
        let store = await makeStore(client: client)

        // Start listening
        let drives = [drive("a", name: "Notes")]
        await store.send(.startListening)

        // Push event through the mock
        client.sendEvent(.drivesChanged(drives))

        await store.receive(\.drivesChanged) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }
    }

    @Test func driveStatusEventFromClient() async {
        let client = MockClient()
        let store = await makeStore(client: client)

        await store.send(.startListening)

        let drives = [drive("a", name: "Notes")]
        client.sendEvent(.drivesChanged(drives))
        await store.receive(\.drivesChanged) { state in
            state.drives = drives
            state.selectedDriveID = "a"
            state.firstRun = nil
        }

        client.sendEvent(.driveStatus(driveID: "a", status: .working))
        await store.receive(\.driveStatus) { state in
            state.drives[0].status = .working
        }
    }
}
