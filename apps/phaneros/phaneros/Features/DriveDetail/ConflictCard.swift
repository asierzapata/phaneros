//
//  ConflictCard.swift
//  phaneros
//
//  A conflict, stated calmly. Nothing was lost — so this must not look like an error.
//

import ComposableArchitecture
import SwiftUI

struct ConflictCard: View {
    let store: StoreOf<DriveDetailFeature>
    let conflict: Conflict

    var body: some View {
        SoftCard {
            HStack(alignment: .top, spacing: 12) {
                PhanerosMark(state: .attention, size: 18)
                    .padding(.top, 2)

                VStack(alignment: .leading, spacing: 4) {
                    Text(conflict.title)
                        .font(.system(size: 13.5, weight: .semibold))
                        .foregroundStyle(Palette.textPrimary)

                    Text(conflict.body)
                        .font(.system(size: 12.5))
                        .foregroundStyle(Palette.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)

                    Button("Show in Finder") {
                        store.send(.revealConflict(conflictID: conflict.id))
                    }
                    .buttonStyle(LinkButtonStyle(size: 12.5))
                    .padding(.top, 5)
                }

                Spacer(minLength: 0)
            }
        }
        .accessibilityElement(children: .combine)
    }
}
