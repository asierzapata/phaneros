//
//  StoreConnection.swift
//  phaneros
//
//  StoreConnection enum representing the connection status to the sync store.
//

import Foundation

nonisolated enum StoreConnection: Hashable {
    case connected(host: String)
    case unreachable(host: String, since: Date)
    case tokenRejected(host: String)
    case notConfigured

    var host: String? {
        switch self {
        case .connected(let h), .unreachable(let h, _), .tokenRejected(let h): h
        case .notConfigured: nil
        }
    }
}
