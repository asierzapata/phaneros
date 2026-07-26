//
//  AddDriveFeatureTests.swift
//  phanerosTests
//
//  Tests for the AddDriveFeature reducer.
//  Written BEFORE the feature exists — they define the expected behavior.
//

import ComposableArchitecture
import Foundation
import Testing

@testable import phaneros

@MainActor
private func makeStore() -> TestStore<AddDriveFeature.State, AddDriveFeature.Action> {
    TestStore(initialState: AddDriveFeature.State()) {
        AddDriveFeature()
    }
}

// MARK: - Initial State

@MainActor
struct AddDriveFeatureInitialStateTests {

    @Test func initialState() async {
        let store = await makeStore()
        await store.send(.binding(.set(\.isPresented, true))) { state in
            state.isPresented = true
            #expect(state.folder == nil)
            #expect(state.name == "")
        }
    }
}

// MARK: - Set Folder

@MainActor
struct AddDriveFeatureSetFolderTests {

    @Test func setFolderUpdatesFolder() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.name = "Notes"
        }
    }

    @Test func setFolderAutoFillsNameWhenEmpty() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.name = "Notes"
        }
    }

    @Test func setFolderDoesNotOverwriteExistingName() async {
        let store = await makeStore()

        // Set a name first
        await store.send(.binding(.set(\.name, "My Drive"))) { state in
            state.name = "My Drive"
        }

        // Now set folder — name should not change
        let folder = URL(fileURLWithPath: "/tmp/Notes")
        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            #expect(state.name == "My Drive")
        }
    }
}

// MARK: - Set Name

@MainActor
struct AddDriveFeatureSetNameTests {

    @Test func setNameUpdatesName() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.name, "Custom Name"))) { state in
            state.name = "Custom Name"
        }
    }
}

// MARK: - Submit

@MainActor
struct AddDriveFeatureSubmitTests {

    @Test func submitSendsAddDriveToParent() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.name = "Notes"
        }

        await store.send(.submitTapped)
        await store.receive(\.delegate.didFinish)
    }

    @Test func submitWithValidDataSucceeds() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/MyDrive")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.name = "MyDrive"
        }

        await store.send(.binding(.set(\.name, "My Drive"))) { state in
            state.name = "My Drive"
        }

        await store.send(.submitTapped)
        await store.receive(\.delegate.didFinish)
    }
}

// MARK: - Cancel

@MainActor
struct AddDriveFeatureCancelTests {

    @Test func cancelDismissesSheet() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.isPresented, true))) { state in
            state.isPresented = true
        }

        await store.send(.cancelTapped) { state in
            state.isPresented = false
            state.folder = nil
            state.name = ""
        }
    }
}

// MARK: - Validation

@MainActor
struct AddDriveFeatureValidationTests {

    @Test func canSubmitWhenFolderAndNameAreSet() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.name = "Notes"
        }

        // Should be able to submit
        await store.send(.submitTapped)
        await store.receive(\.delegate.didFinish)
    }

    @Test func cannotSubmitWithoutFolder() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.name, "My Drive"))) { state in
            state.name = "My Drive"
        }

        // Submit should not send to parent without folder
        await store.send(.submitTapped)
    }

    @Test func cannotSubmitWithoutName() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.name = "Notes"
        }

        // Clear the auto-filled name
        await store.send(.binding(.set(\.name, ""))) { state in
            state.name = ""
        }

        // Submit should not send to parent without name
        await store.send(.submitTapped)
    }
}
