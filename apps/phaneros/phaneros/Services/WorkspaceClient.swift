//
//  WorkspaceClient.swift
//  phaneros
//
//  A dependency for workspace operations (Finder, etc.) to enable testing.
//

import ComposableArchitecture
import Foundation

struct WorkspaceClient {
    var revealInFinder: @Sendable (URL) -> Void = { _ in }
}

extension WorkspaceClient: TestDependencyKey {
    static let previewValue = WorkspaceClient()
    static let testValue = WorkspaceClient()
}

extension DependencyValues {
    var workspace: WorkspaceClient {
        get { self[WorkspaceClient.self] }
        set { self[WorkspaceClient.self] = newValue }
    }
}
