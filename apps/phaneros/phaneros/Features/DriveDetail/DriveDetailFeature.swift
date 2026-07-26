//
//  DriveDetailFeature.swift
//  phaneros
//
//  The drive detail view logic extracted into its own feature.
//

import ComposableArchitecture
import Foundation

@Reducer
struct DriveDetailFeature {
    @ObservableState
    struct State: Equatable {
        var drive: Drive
    }

    enum Action: Equatable {
        case pauseTapped
        case resumeTapped
        case revealInFinder
        case revealConflict(conflictID: UUID)
        case delegate(Delegate)

        @CasePathable
        enum Delegate: Equatable {
            case pauseDrive(driveID: String)
            case resumeDrive(driveID: String)
            case didRevealInFinder(url: URL)
            case didRevealConflict(conflictID: UUID)
        }
    }

    @Dependency(\.workspace) var workspace

    var body: some Reducer<State, Action> {
        Reduce { state, action in
            switch action {
            case .pauseTapped:
                return .send(.delegate(.pauseDrive(driveID: state.drive.id)))

            case .resumeTapped:
                return .send(.delegate(.resumeDrive(driveID: state.drive.id)))

            case .revealInFinder:
                let url = state.drive.path
                workspace.revealInFinder(url)
                return .send(.delegate(.didRevealInFinder(url: url)))

            case .revealConflict(let conflictID):
                guard let conflict = state.drive.conflicts.first(where: { $0.id == conflictID })
                else {
                    return .none
                }
                workspace.revealInFinder(conflict.fileURL)
                return .send(.delegate(.didRevealConflict(conflictID: conflictID)))

            case .delegate:
                return .none
            }
        }
    }
}
