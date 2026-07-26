import ComposableArchitecture
import Foundation
import Testing

@testable import phaneros

@MainActor
private func makeStore() -> TestStore<FirstRunFeature.State, FirstRunFeature.Action> {
    TestStore(initialState: FirstRunFeature.State()) {
        FirstRunFeature()
    }
}

// MARK: - Initial State

@MainActor
struct FirstRunFeatureInitialStateTests {

    @Test func initialState() async {
        let store = await makeStore()
        await store.send(.binding(.set(\.isPresented, true))) { state in
            state.isPresented = true
            #expect(state.step == .folder)
            #expect(state.folder == nil)
            #expect(state.driveName == "")
            #expect(state.storeHost == "")
            #expect(state.token == "")
        }
    }
}

// MARK: - Step Transitions

@MainActor
struct FirstRunFeatureStepTransitionTests {

    @Test func nextStepFromFolderToStore() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.driveName = "Notes"
        }

        await store.send(.nextStepTapped) { state in
            state.step = .store
        }
    }

    @Test func nextStepFromStoreToConfirm() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.step, .store))) { state in
            state.step = .store
        }
        await store.send(.binding(.set(\.storeHost, "phaneros.example.com"))) { state in
            state.storeHost = "phaneros.example.com"
        }
        await store.send(.binding(.set(\.token, "abc123"))) { state in
            state.token = "abc123"
        }

        await store.send(.nextStepTapped) { state in
            state.step = .confirm
        }
    }

    @Test func previousStepFromStoreToFolder() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.step, .store))) { state in
            state.step = .store
        }

        await store.send(.previousStepTapped) { state in
            state.step = .folder
        }
    }

    @Test func previousStepFromConfirmToStore() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.step, .confirm))) { state in
            state.step = .confirm
        }

        await store.send(.previousStepTapped) { state in
            state.step = .store
        }
    }

    @Test func previousStepFromFolderDoesNothing() async {
        let store = await makeStore()

        await store.send(.previousStepTapped)
        #expect(store.state.step == .folder)
    }
}

// MARK: - Validation

@MainActor
struct FirstRunFeatureValidationTests {

    @Test func cannotAdvanceFromFolderWithoutFolder() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.driveName, "Notes"))) { state in
            state.driveName = "Notes"
        }

        await store.send(.nextStepTapped)
        #expect(store.state.step == .folder)
    }

    @Test func cannotAdvanceFromFolderWithoutName() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.driveName = "Notes"
        }
        await store.send(.binding(.set(\.driveName, ""))) { state in
            state.driveName = ""
        }

        await store.send(.nextStepTapped)
        #expect(store.state.step == .folder)
    }

    @Test func cannotAdvanceFromStoreWithoutHost() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.step, .store))) { state in
            state.step = .store
        }
        await store.send(.binding(.set(\.token, "abc123"))) { state in
            state.token = "abc123"
        }

        await store.send(.nextStepTapped)
        #expect(store.state.step == .store)
    }

    @Test func cannotAdvanceFromStoreWithoutToken() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.step, .store))) { state in
            state.step = .store
        }
        await store.send(.binding(.set(\.storeHost, "phaneros.example.com"))) { state in
            state.storeHost = "phaneros.example.com"
        }

        await store.send(.nextStepTapped)
        #expect(store.state.step == .store)
    }
}

// MARK: - Folder Auto-Fill

@MainActor
struct FirstRunFeatureFolderAutoFillTests {

    @Test func folderAutoFillsNameWhenEmpty() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.driveName = "Notes"
        }
    }

    @Test func folderDoesNotOverwriteExistingName() async {
        let store = await makeStore()

        await store.send(.binding(.set(\.driveName, "My Drive"))) { state in
            state.driveName = "My Drive"
        }

        let folder = URL(fileURLWithPath: "/tmp/Notes")
        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            #expect(state.driveName == "My Drive")
        }
    }
}

// MARK: - Submit

@MainActor
struct FirstRunFeatureSubmitTests {

    @Test func submitSendsDelegateToParent() async {
        let store = await makeStore()
        let folder = URL(fileURLWithPath: "/tmp/Notes")

        await store.send(.binding(.set(\.folder, folder))) { state in
            state.folder = folder
            state.driveName = "Notes"
        }
        await store.send(.binding(.set(\.step, .confirm))) { state in
            state.step = .confirm
        }

        await store.send(.delegate(.didFinish(name: "Notes", path: folder)))
    }
}
