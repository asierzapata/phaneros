//
//  RemoveDriveFeatureTests.swift
//  phanerosTests
//
//  Tests for the RemoveDriveFeature reducer.
//  Written BEFORE the feature exists — they define the expected behavior.
//

import ComposableArchitecture
import Foundation
import Testing

@testable import phaneros

@MainActor
private func makeStore() -> TestStore<RemoveDriveFeature.State, RemoveDriveFeature.Action> {
    TestStore(initialState: RemoveDriveFeature.State()) {
        RemoveDriveFeature()
    }
}

@MainActor
private func testDrive() -> Drive {
    Drive(
        id: "a",
        name: "Notes",
        path: URL(fileURLWithPath: "/tmp/Notes"),
        status: .upToDate(at: .now)
    )
}

// MARK: - Initial State

@MainActor
struct RemoveDriveFeatureInitialStateTests {

    @Test func initialState() async {
        let store = await makeStore()
        #expect(store.state.drive == nil)
        #expect(store.state.isPresented == false)
    }
}

// MARK: - Confirm

@MainActor
struct RemoveDriveFeatureConfirmTests {

    @Test func confirmSendsDelegateToParent() async {
        let store = await makeStore()
        let drive = testDrive()

        await store.send(.binding(.set(\.drive, drive))) { state in
            state.drive = drive
        }

        await store.send(.confirmTapped)
        await store.receive(\.delegate.didConfirmRemoval, drive.id)
    }

    @Test func confirmWithoutDriveDoesNothing() async {
        let store = await makeStore()

        await store.send(.confirmTapped)
    }
}

// MARK: - Cancel

@MainActor
struct RemoveDriveFeatureCancelTests {

    @Test func cancelResetsState() async {
        let store = await makeStore()
        let drive = testDrive()

        await store.send(.binding(.set(\.drive, drive))) { state in
            state.drive = drive
        }

        await store.send(.cancelTapped) { state in
            state.drive = nil
        }
    }
}
