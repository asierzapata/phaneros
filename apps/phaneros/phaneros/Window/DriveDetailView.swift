//
//  DriveDetailView.swift
//  phaneros
//
//  One drive: what state it's in, when it last synced, where it lives, and a way in.
//  Recent activity is phrased the way a person would say it — never as transfer
//  telemetry, per documentation/ui-v1.md.
//

import ComposableArchitecture
import SwiftUI

struct DriveDetailView: View {
    let store: StoreOf<DriveDetailFeature>
    let catchUp: AppFeature.CatchUp?

    var body: some View {
        ScrollViewIfNeeded {
            VStack(alignment: .leading, spacing: 0) {
                header

                Text(subtitle)
                    .font(.system(size: 13.5))
                    .foregroundStyle(Palette.textQuaternary)
                    .padding(.top, 6)

                if !store.state.drive.conflicts.isEmpty {
                    VStack(spacing: 10) {
                        ForEach(store.state.drive.conflicts) { conflict in
                            ConflictCard(store: store, conflict: conflict)
                        }
                    }
                    .padding(.top, 24)
                }

                if let catchUp {
                    SoftCard {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Catching up")
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(Palette.textPrimary)
                            Text(
                                "\(catchUp.device) made changes while this Mac was asleep. \(catchUp.fileCount.formatted(.number)) files coming in."
                            )
                            .font(.system(size: 12.5))
                            .foregroundStyle(Palette.textTertiary)
                        }
                    }
                    .padding(.top, 16)
                }

                Text("Recent activity")
                    .sectionLabelStyle()
                    .padding(.top, 28)
                    .padding(.bottom, 12)

                if store.state.drive.activity.isEmpty {
                    Text(
                        "Nothing yet. When files change here or on another device, you'll see it listed."
                    )
                    .font(.system(size: 13.5))
                    .foregroundStyle(Palette.textTertiary)
                    .frame(maxWidth: 420, alignment: .leading)
                } else {
                    VStack(alignment: .leading, spacing: 11) {
                        ForEach(store.state.drive.activity) { entry in
                            Text(entry.line)
                                .font(.system(size: 14.5))
                                .foregroundStyle(Palette.textSecondary)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                }

                Spacer(minLength: 20)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 34)
            .padding(.vertical, 28)
        }
        .background(Palette.card)
    }

    private var header: some View {
        HStack(alignment: .top) {
            Text(store.state.drive.name)
                .font(.system(size: 24, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)

            Spacer()

            HStack(spacing: 8) {
                Button(store.state.drive.status == .paused ? "Resume" : "Pause") {
                    if store.state.drive.status == .paused {
                        store.send(.resumeTapped)
                    } else {
                        store.send(.pauseTapped)
                    }
                }
                .buttonStyle(SecondaryButtonStyle())

                Button("Open in Finder") {
                    store.send(.revealInFinder)
                }
                .buttonStyle(PrimaryButtonStyle())
            }
        }
    }

    /// "Up to date · synced just now · ~/Documents/Notes"
    private var subtitle: String {
        var parts = [store.state.drive.status.shortLabel]
        if case .upToDate(let at) = store.state.drive.status {
            parts.append("synced \(at.phanerosRelative)")
        }
        parts.append(store.state.drive.displayPath)
        return parts.joined(separator: " · ")
    }
}
