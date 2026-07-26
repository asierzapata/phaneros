import ComposableArchitecture
import Foundation

@Reducer
struct FirstRunFeature {
    @ObservableState
    struct State: Equatable {
        enum Step: Int, Equatable {
            case folder = 1
            case store = 2
            case confirm = 3
        }

        var driveName = ""
        var folder: URL?
        var isPresented = false
        var step: Step = .folder
        var storeHost = ""
        var token = ""

        var canAdvanceFromFolder: Bool {
            folder != nil && !driveName.isEmpty
        }

        var canAdvanceFromStore: Bool {
            !storeHost.isEmpty && !token.isEmpty
        }
    }

    enum Action: BindableAction {
        case binding(BindingAction<State>)
        case cancelTapped
        case delegate(Delegate)
        case nextStepTapped
        case previousStepTapped

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
                if state.driveName.isEmpty, let folder = state.folder {
                    state.driveName = folder.lastPathComponent
                }
                return .none

            case .binding:
                return .none

            case .cancelTapped:
                return .none

            case .nextStepTapped:
                switch state.step {
                case .folder:
                    guard state.canAdvanceFromFolder else { return .none }
                    state.step = .store
                case .store:
                    guard state.canAdvanceFromStore else { return .none }
                    state.step = .confirm
                case .confirm:
                    break
                }
                return .none

            case .previousStepTapped:
                switch state.step {
                case .folder:
                    break
                case .store:
                    state.step = .folder
                case .confirm:
                    state.step = .store
                }
                return .none

            case .delegate(.didFinish):
                return .none
            }
        }
    }
}
