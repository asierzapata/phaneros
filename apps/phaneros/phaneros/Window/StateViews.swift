//
//  StateViews.swift
//  phaneros
//
//  Job #7: the moments where things aren't fine. A sync app is judged almost
//  entirely on these, so each one gets the whole detail pane rather than a banner.
//
//  The register throughout: say what happened, say the files are safe, and only ask
//  for something when there is genuinely something to do.
//

import ComposableArchitecture
import SwiftUI

// MARK: - Empty

/// Nothing has happened yet — first launch, no drives.
struct EmptyStateView: View {
    @Bindable var store: StoreOf<AppFeature>

    var body: some View {
        VStack(spacing: 0) {
            PhanerosMark(state: .empty, size: 40)
                .padding(.bottom, 16)

            Text("No drives yet")
                .font(.system(size: 19, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 8)

            Text(
                "Point Phaneros at a folder and it'll follow you everywhere else you're signed in."
            )
            .font(.system(size: 13.5))
            .foregroundStyle(Palette.textTertiary)
            .multilineTextAlignment(.center)
            .frame(maxWidth: 340)
            .padding(.bottom, 22)

            Button("Add your first drive") { store.send(.addDriveTapped) }
                .buttonStyle(PrimaryButtonStyle())
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Palette.card)
    }
}

// MARK: - First sync

/// The one time a progress indicator is honestly earned.
struct FirstSyncView: View {
    var drive: Drive
    var progress: FirstSyncProgress

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Bringing \"\(drive.name)\" over")
                .font(.system(size: 22, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 8)

            Text("This only happens once — after this, only changes move.")
                .font(.system(size: 14))
                .foregroundStyle(Palette.textTertiary)
                .padding(.bottom, 28)

            ProgressView(value: progress.fraction)
                .progressViewStyle(PhanerosProgressStyle())
                .frame(maxWidth: 520)
                .padding(.bottom, 12)

            Text(progress.caption)
                .font(.system(size: 13))
                .foregroundStyle(Palette.textQuaternary)

            Spacer()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 60)
        .padding(.vertical, 60)
        .background(Palette.card)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Bringing \(drive.name) over. \(progress.caption)")
    }
}

/// A flat warm track with a terracotta fill — no gloss, no stripes.
struct PhanerosProgressStyle: ProgressViewStyle {
    func makeBody(configuration: Configuration) -> some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                Capsule().fill(Palette.sunken)
                Capsule()
                    .fill(Palette.accent)
                    .frame(width: proxy.size.width * (configuration.fractionCompleted ?? 0))
            }
        }
        .frame(height: 10)
        .animation(.easeOut(duration: 0.3), value: configuration.fractionCompleted ?? 0)
    }
}

// MARK: - Offline

/// The store is unreachable. There is nothing for the user to do, so don't imply there is.
struct StoreUnreachableView: View {
    var host: String

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Can't reach \(host)")
                .font(.system(size: 20, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 10)

            Text(
                "Your files are safe right where they are on this Mac. Phaneros will pick up the moment the store is back — nothing to do here."
            )
            .font(.system(size: 14))
            .foregroundStyle(Palette.textTertiary)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: 460, alignment: .leading)

            Spacer()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(60)
        .background(Palette.card)
    }
}

// MARK: - Token rejected

/// The one bad state that does have an action attached.
struct ReconnectView: View {
    @Bindable var store: StoreOf<AppFeature>
    var host: String

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("\(host) needs you to reconnect")
                .font(.system(size: 20, weight: .semibold))
                .foregroundStyle(Palette.textPrimary)
                .padding(.bottom, 10)

            Text(
                "Nothing new will sync until then. Your files are untouched, and nothing has been removed from this Mac."
            )
            .font(.system(size: 14))
            .foregroundStyle(Palette.textTertiary)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: 460, alignment: .leading)
            .padding(.bottom, 22)

            Button("Reconnect") { store.send(.firstRunTapped) }
                .buttonStyle(PrimaryButtonStyle())

            Spacer()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(60)
        .background(Palette.card)
    }
}
