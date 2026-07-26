//
//  MainWindowView.swift
//  phaneros
//

import ComposableArchitecture
import SwiftUI

struct MainWindowView: View {
    @Bindable var store: StoreOf<AppFeature>

    var body: some View {
        Group {
            if store.state.drives.isEmpty {
                EmptyStateView(store: store)
            } else {
                NavigationSplitView {
                    DriveSidebar(store: store)
                        .navigationSplitViewColumnWidth(min: 210, ideal: 250, max: 300)
                } detail: {
                    detail
                        .frame(minWidth: 460, minHeight: 380)
                }
                .navigationSplitViewStyle(.balanced)
            }
        }
        .frame(minWidth: 720, minHeight: 460)
        .background(Palette.card)
        .sheet(item: $store.scope(state: \.addDrive, action: \.addDrive)) { addDriveStore in
            AddDriveSheet(store: addDriveStore)
        }
        .sheet(item: $store.scope(state: \.firstRun, action: \.firstRun)) { firstRunStore in
            FirstRunView(store: firstRunStore)
        }
        .sheet(item: $store.scope(state: \.removeDrive, action: \.removeDrive)) { removeDriveStore in
            RemoveDriveSheet(store: removeDriveStore)
        }
    }

    @ViewBuilder
    private var detail: some View {
        if let drive = store.state.selectedDrive {
            if let progress = drive.firstSync {
                FirstSyncView(drive: drive, progress: progress)
            } else if case .unreachable(let host, _) = store.state.connection {
                StoreUnreachableView(host: host)
            } else if case .tokenRejected(let host) = store.state.connection {
                ReconnectView(store: store, host: host)
            } else if let driveDetailStore = store.scope(
                state: \.driveDetail,
                action: \.driveDetail
            ) {
                DriveDetailView(store: driveDetailStore, catchUp: store.state.catchUp)
            }
        } else {
            Color.clear
        }
    }
}
