//
//  RemoveDriveFeature.swift
//  phaneros
//
//  The "Remove Drive" confirmation sheet logic extracted into its own feature.
//

import ComposableArchitecture
import Foundation

@Reducer
struct RemoveDriveFeature {
    @ObservableState
    struct State: Equatable {
        var drive: Drive?
        var isPresented: Bool = false
    }

    enum Action: BindableAction {
        case binding(BindingAction<State>)
        case confirmTapped
        case cancelTapped
        case delegate(Delegate)

        @CasePathable
        enum Delegate: Equatable {
            case didConfirmRemoval(driveID: String)
        }
    }

    var body: some Reducer<State, Action> {
        BindingReducer()
        Reduce { state, action in
            switch action {
            case .binding:
                return .none

            case .confirmTapped:
                guard let driveID = state.drive?.id else {
                    return .none
                }
                return .send(.delegate(.didConfirmRemoval(driveID: driveID)))

            case .cancelTapped:
                state.drive = nil
                return .none

            case .delegate:
                return .none
            }
        }
    }
}
