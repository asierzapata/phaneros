//
//  DriveDetailFeatureTests.swift
//  phanerosTests
//
//  Tests for the DriveDetailFeature reducer.
//  Written BEFORE the feature exists — they define the expected behavior.
//

import ComposableArchitecture
import Foundation
import Testing

@testable import phaneros

@MainActor
private func makeStore(
    drive: Drive = .preview
) -> TestStore<DriveDetailFeature.State, DriveDetailFeature.Action> {
    TestStore(initialState: DriveDetailFeature.State(drive: drive)) {
        DriveDetailFeature()
    }
}

// MARK: - Initial State

@MainActor
struct DriveDetailFeatureInitialStateTests {

    @Test func initialStateHoldsDrive() async {
        let drive = Drive.preview
        let store = await makeStore(drive: drive)

        #expect(store.state.drive == drive)
        #expect(store.state.drive.id == drive.id)
        #expect(store.state.drive.name == drive.name)
    }
}

// MARK: - Pause

@MainActor
struct DriveDetailFeaturePauseTests {

    @Test func pauseTappedSendsDelegateToParent() async {
        let drive = Drive.preview
        let store = await makeStore(drive: drive)

        // State doesn't change; delegate is sent
        await store.send(.pauseTapped)

        await store.receive(.delegate(.pauseDrive(driveID: drive.id)))
    }

    @Test func pauseTappedWithDifferentDriveID() async {
        let drive = Drive.preview(id: "custom-drive-id")
        let store = await makeStore(drive: drive)

        await store.send(.pauseTapped)
        await store.receive(.delegate(.pauseDrive(driveID: "custom-drive-id")))
    }
}

// MARK: - Resume

@MainActor
struct DriveDetailFeatureResumeTests {

    @Test func resumeTappedSendsDelegateToParent() async {
        let drive = Drive.preview
        let store = await makeStore(drive: drive)

        // State doesn't change; delegate is sent
        await store.send(.resumeTapped)

        await store.receive(.delegate(.resumeDrive(driveID: drive.id)))
    }

    @Test func resumeTappedWithDifferentDriveID() async {
        let drive = Drive.preview(id: "another-drive")
        let store = await makeStore(drive: drive)

        await store.send(.resumeTapped)
        await store.receive(.delegate(.resumeDrive(driveID: "another-drive")))
    }
}

// MARK: - Reveal in Finder

@MainActor
struct DriveDetailFeatureRevealTests {

    @Test func revealInFinderCallsWorkspace() async {
        let drive = Drive.preview
        var workspaceCalls: [URL] = []

        let store = await TestStore(
            initialState: DriveDetailFeature.State(drive: drive)
        ) {
            DriveDetailFeature()
        } withDependencies: {
            $0.workspace.revealInFinder = { url in
                workspaceCalls.append(url)
            }
        }

        await store.send(.revealInFinder)
        await store.receive(.delegate(.didRevealInFinder(url: drive.path)))

        #expect(workspaceCalls.count == 1)
        #expect(workspaceCalls.first == drive.path)
    }

    @Test func revealConflictCallsWorkspace() async {
        var drive = Drive.preview
        let conflict = Conflict.preview
        drive.conflicts = [conflict]

        var workspaceCalls: [URL] = []

        let store = await TestStore(
            initialState: DriveDetailFeature.State(drive: drive)
        ) {
            DriveDetailFeature()
        } withDependencies: {
            $0.workspace.revealInFinder = { url in
                workspaceCalls.append(url)
            }
        }

        await store.send(.revealConflict(conflictID: conflict.id))
        await store.receive(.delegate(.didRevealConflict(conflictID: conflict.id)))

        #expect(workspaceCalls.count == 1)
        #expect(workspaceCalls.first == conflict.fileURL)
    }

    @Test func revealConflictWithInvalidIDDoesNothing() async {
        let drive = Drive.preview
        let store = await makeStore(drive: drive)

        await store.send(.revealConflict(conflictID: UUID()))
        // No effect should be triggered
    }
}

// MARK: - Delegate

@MainActor
struct DriveDetailFeatureDelegateTests {

    @Test func delegateActionsDoNotChangeState() async {
        let drive = Drive.preview
        let store = await makeStore(drive: drive)

        // Delegates are for the parent; they leave this feature's state alone.
        await store.send(.delegate(.pauseDrive(driveID: drive.id)))
        #expect(store.state.drive == drive)

        await store.send(.delegate(.resumeDrive(driveID: drive.id)))
        #expect(store.state.drive == drive)
    }
}

// MARK: - Preview Data

extension Drive {
    static var preview: Drive { preview(id: "preview-drive") }

    static func preview(id: String) -> Drive {
        Drive(
            id: id,
            name: "Preview Drive",
            path: URL(fileURLWithPath: "/tmp/Preview"),
            status: .upToDate(at: .now),
            activity: [],
            conflicts: [],
            firstSync: nil
        )
    }
}

extension Conflict {
    static var preview: Conflict {
        Conflict(
            fileName: "test.txt",
            keptCopyName: "test (kept).txt",
            otherDevice: "Other Mac",
            kind: .bothEdited,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/test.txt")
        )
    }
}
