//
//  GlanceFeatureTests.swift
//  phanerosTests
//
//  Tests for the GlanceFeature reducer.
//

import CasePaths
import ComposableArchitecture
import Foundation
import Testing

@testable import phaneros

@MainActor
private func makeStore() -> TestStore<GlanceFeature.State, GlanceFeature.Action> {
    TestStore(initialState: GlanceFeature.State()) {
        GlanceFeature()
    }
}

// MARK: - Initial State

@MainActor
struct GlanceFeatureInitialStateTests {

    @Test func initialState() async {
        let store = await makeStore()
        #expect(store.state.drives.isEmpty)
        #expect(store.state.connection == .notConfigured)
        #expect(store.state.pauseAllSyncing == false)
        #expect(store.state.overallMark == .empty)
        #expect(store.state.overallSummary == "No drives yet")
        #expect(store.state.recentActivity.isEmpty)
        #expect(store.state.allConflicts.isEmpty)
    }
}

// MARK: - Open Phaneros

@MainActor
struct GlanceFeatureOpenPhanerosTests {

    @Test func openPhanerosSendsDelegate() async {
        let store = await makeStore()
        await store.send(.openPhanerosTapped)
        await store.receive(\.delegate.openMainWindow)
    }
}

// MARK: - Pause/Resume All

@MainActor
struct GlanceFeaturePauseResumeTests {

    @Test func pauseAllSendsDelegate() async {
        let store = await makeStore()
        await store.send(.pauseAllTapped)
        await store.receive(\.delegate.togglePauseAll)
    }

    @Test func resumeAllSendsDelegate() async {
        let store = await makeStore()
        await store.send(.resumeAllTapped)
        await store.receive(\.delegate.togglePauseAll)
    }
}

// MARK: - Reveal Drive

@MainActor
struct GlanceFeatureRevealDriveTests {

    @Test func revealDriveSendsDelegate() async {
        let store = await makeStore()
        let driveID = "test-drive-1"

        await store.send(.revealDriveTapped(driveID: driveID))
        await store.receive(\.delegate.revealDriveInFinder, driveID)
    }
}

// MARK: - Reveal Conflict

@MainActor
struct GlanceFeatureRevealConflictTests {

    @Test func revealConflictSendsDelegate() async {
        let store = await makeStore()
        let conflict = Conflict(
            fileName: "test.txt",
            keptCopyName: "test (conflict).txt",
            otherDevice: "Other Mac",
            kind: .bothEdited,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/test.txt")
        )

        await store.send(.revealConflictTapped(conflict: conflict))
        await store.receive(\.delegate.revealConflictInFinder, conflict)
    }
}

// MARK: - Quit

@MainActor
struct GlanceFeatureQuitTests {

    @Test func quitSendsDelegate() async {
        let store = await makeStore()
        await store.send(.quitTapped)
        await store.receive(\.delegate.quitApp)
    }
}

// MARK: - Derived State

@MainActor
struct GlanceFeatureDerivedStateTests {

    @Test func overallMarkWithDrives() async {
        let store = await makeStore()
        let drive = Drive(
            id: "test-drive-1",
            name: "Test Drive",
            path: URL(fileURLWithPath: "/tmp/test"),
            status: .upToDate(at: .now),
            activity: [],
            conflicts: [],
            firstSync: nil
        )

        await store.send(.binding(.set(\.drives, [drive]))) { state in
            state.drives = [drive]
        }

        #expect(store.state.overallMark == .upToDate)
    }

    @Test func overallMarkWithUnreachableConnection() async {
        let store = await makeStore()
        let drive = Drive(
            id: "test-drive-1",
            name: "Test Drive",
            path: URL(fileURLWithPath: "/tmp/test"),
            status: .upToDate(at: .now),
            activity: [],
            conflicts: [],
            firstSync: nil
        )

        await store.send(.binding(.set(\.drives, [drive]))) { state in
            state.drives = [drive]
        }

        let since = Date.now
        await store.send(.binding(.set(\.connection, .unreachable(host: "example.com", since: since)))) { state in
            state.connection = .unreachable(host: "example.com", since: since)
        }

        #expect(store.state.overallMark == .offline)
    }

    @Test func overallSummaryWithDrives() async {
        let store = await makeStore()
        let drive = Drive(
            id: "test-drive-1",
            name: "Test Drive",
            path: URL(fileURLWithPath: "/tmp/test"),
            status: .upToDate(at: .now),
            activity: [],
            conflicts: [],
            firstSync: nil
        )

        await store.send(.binding(.set(\.drives, [drive]))) { state in
            state.drives = [drive]
        }

        #expect(store.state.overallSummary == "Up to date")
    }

    @Test func recentActivityReturnsTopThree() async {
        let store = await makeStore()
        let entries = (1...5).map { i in
            ActivityEntry(
                text: "Entry \(i)",
                at: Date().addingTimeInterval(TimeInterval(-i))
            )
        }
        let drive = Drive(
            id: "test-drive-1",
            name: "Test Drive",
            path: URL(fileURLWithPath: "/tmp/test"),
            status: .upToDate(at: .now),
            activity: entries,
            conflicts: [],
            firstSync: nil
        )

        await store.send(.binding(.set(\.drives, [drive]))) { state in
            state.drives = [drive]
        }

        #expect(store.state.recentActivity.count == 3)
        #expect(store.state.recentActivity[0].text == "Entry 1")
        #expect(store.state.recentActivity[1].text == "Entry 2")
        #expect(store.state.recentActivity[2].text == "Entry 3")
    }

    @Test func allConflictsFlattensFromAllDrives() async {
        let store = await makeStore()
        let conflict1 = Conflict(
            fileName: "test1.txt",
            keptCopyName: "test1 (conflict).txt",
            otherDevice: "Other Mac",
            kind: .bothEdited,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/test1.txt")
        )
        let conflict2 = Conflict(
            fileName: "test2.txt",
            keptCopyName: "test2 (conflict).txt",
            otherDevice: "Other Mac",
            kind: .editedAndDeleted,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/test2.txt")
        )
        let drive1 = Drive(
            id: "test-drive-1",
            name: "Test Drive 1",
            path: URL(fileURLWithPath: "/tmp/test1"),
            status: .needsAttention(reason: "conflict"),
            activity: [],
            conflicts: [conflict1],
            firstSync: nil
        )
        let drive2 = Drive(
            id: "test-drive-2",
            name: "Test Drive 2",
            path: URL(fileURLWithPath: "/tmp/test2"),
            status: .needsAttention(reason: "conflict"),
            activity: [],
            conflicts: [conflict2],
            firstSync: nil
        )

        await store.send(.binding(.set(\.drives, [drive1, drive2]))) { state in
            state.drives = [drive1, drive2]
        }

        #expect(store.state.allConflicts.count == 2)
    }
}
