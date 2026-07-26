//
//  AddDriveFeature.swift
//  phaneros
//
//  The "Add Drive" sheet logic extracted into its own feature.
//

import ComposableArchitecture
import Foundation

@Reducer
struct AddDriveFeature {
    @ObservableState
    struct State: Equatable {
        var folder: URL?
        var name: String = ""
        var isPresented: Bool = false
    }

    enum Action: BindableAction {
        case binding(BindingAction<State>)
        case submitTapped
        case cancelTapped
        case delegate(Delegate)

        @CasePathable
        enum Delegate: Equatable {
            case didFinish(name: String, path: URL)
        }
    }

    var body: some Reducer<State, Action> {
        BindingReducer()
        Reduce { state, action in
            switch action {
            case .binding(\.folder):
                if state.name.isEmpty, let folder = state.folder {
                    state.name = folder.lastPathComponent
                }
                return .none

            case .binding:
                return .none

            case .submitTapped:
                guard let folder = state.folder, !state.name.isEmpty else {
                    return .none
                }
                return .send(.delegate(.didFinish(name: state.name, path: folder)))

            case .cancelTapped:
                state.isPresented = false
                state.folder = nil
                state.name = ""
                return .none

            case .delegate:
                return .none
            }
        }
    }
}
