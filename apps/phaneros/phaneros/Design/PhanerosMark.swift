//
//  PhanerosMark.swift
//  phaneros
//
//  The concentric-ring mark, in each of the five states the engine can be in.
//
//  Geometry is transcribed from the design's 24x24 viewBox: a filled core at r=4.2
//  and a ring at r=9. Everything below is expressed as a fraction of that box so the
//  mark is identical at 13pt in a list row and at 40pt in an empty state.
//

import SwiftUI

enum MarkState: Hashable {
    /// Solid core, nothing around it. The good state, and the quietest.
    case upToDate
    /// Core plus a ring that turns. The only motion in the whole app.
    case syncing
    /// Ring with two bars inside it. Deliberately hollow — nothing is happening.
    case paused
    /// Core plus ring, in amber. "Needs a look", never "error".
    case attention
    /// Dashed and hollow. The store is out of reach; the files are still fine.
    case offline
    /// No drives yet. A dashed core on its own.
    case empty

    var tint: Color {
        switch self {
        case .upToDate, .syncing, .empty: Palette.accent
        case .attention: Palette.attention
        case .paused, .offline: Palette.dormant
        }
    }
}

struct PhanerosMark: View {
    var state: MarkState
    var size: CGFloat = 16
    /// Set false for static renders (menu bar snapshots, previews in a list).
    var animated: Bool = true

    /// Fractions of the 24pt design box.
    private var scale: CGFloat { size / 24 }
    private var core: CGFloat { 8.4 * scale }      // r = 4.2
    private var ring: CGFloat { 18 * scale }       // r = 9
    private var ringWidth: CGFloat { 1.3 * scale }
    private var hairWidth: CGFloat { 1.4 * scale }
    private var dash: [CGFloat] { [2 * scale, 3 * scale] }

    @State private var spin = false

    var body: some View {
        ZStack {
            switch state {
            case .upToDate:
                Circle()
                    .fill(state.tint)
                    .frame(width: core, height: core)

            case .syncing:
                Circle()
                    .fill(state.tint)
                    .frame(width: core, height: core)
                Circle()
                    .stroke(state.tint.opacity(0.5), lineWidth: ringWidth)
                    .frame(width: ring, height: ring)
                Circle()
                    .trim(from: 0, to: 0.28)
                    .stroke(state.tint, style: StrokeStyle(lineWidth: ringWidth, lineCap: .round))
                    .frame(width: ring, height: ring)
                    .rotationEffect(.degrees(spin ? 360 : 0))

            case .paused:
                Circle()
                    .stroke(state.tint, lineWidth: hairWidth)
                    .frame(width: ring, height: ring)
                HStack(spacing: 2.4 * scale) {
                    ForEach(0..<2, id: \.self) { _ in
                        Capsule()
                            .fill(Palette.textSecondary)
                            .frame(width: 1.6 * scale, height: 6 * scale)
                    }
                }

            case .attention:
                Circle()
                    .fill(state.tint)
                    .frame(width: core, height: core)
                Circle()
                    .stroke(state.tint.opacity(0.6), lineWidth: ringWidth)
                    .frame(width: ring, height: ring)

            case .offline:
                Circle()
                    .stroke(state.tint, lineWidth: hairWidth)
                    .frame(width: core, height: core)
                Circle()
                    .stroke(
                        state.tint.opacity(0.6),
                        style: StrokeStyle(lineWidth: 1.2 * scale, dash: dash)
                    )
                    .frame(width: ring, height: ring)

            case .empty:
                Circle()
                    .stroke(
                        state.tint,
                        style: StrokeStyle(lineWidth: 1.5 * scale, dash: dash)
                    )
                    .frame(width: core, height: core)
            }
        }
        .frame(width: size, height: size)
        .onAppear {
            guard animated, state == .syncing else { return }
            withAnimation(.linear(duration: 1.4).repeatForever(autoreverses: false)) {
                spin = true
            }
        }
        .onChange(of: state) { _, new in
            guard animated else { return }
            spin = false
            guard new == .syncing else { return }
            withAnimation(.linear(duration: 1.4).repeatForever(autoreverses: false)) {
                spin = true
            }
        }
        .accessibilityHidden(true)
    }
}

extension PhanerosMark {
    /// A non-template `NSImage` for the menu bar.
    ///
    /// The mark is deliberately *not* a template image — the design reserves colour
    /// for the states that need to be noticed, and templating would flatten all five
    /// into the same monochrome glyph.
    @MainActor
    static func menuBarImage(for state: MarkState, appearance: NSAppearance?) -> NSImage {
        let renderer = ImageRenderer(
            content: PhanerosMark(state: state, size: 17, animated: false)
                .padding(1)
                .environment(\.colorScheme, appearance?.isDark == true ? .dark : .light)
        )
        renderer.scale = 3
        let image = renderer.nsImage ?? NSImage(size: NSSize(width: 19, height: 19))
        image.isTemplate = false
        image.accessibilityDescription = state.accessibilityLabel
        return image
    }
}

extension NSAppearance {
    var isDark: Bool {
        bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
    }
}

extension MarkState {
    var accessibilityLabel: String {
        switch self {
        case .upToDate: "Phaneros — up to date"
        case .syncing: "Phaneros — syncing"
        case .paused: "Phaneros — paused"
        case .attention: "Phaneros — needs attention"
        case .offline: "Phaneros — can't reach the store"
        case .empty: "Phaneros — no drives yet"
        }
    }
}

#Preview("Mark states") {
    HStack(spacing: 22) {
        ForEach(
            [MarkState.upToDate, .syncing, .paused, .attention, .offline, .empty],
            id: \.self
        ) { state in
            VStack(spacing: 10) {
                PhanerosMark(state: state, size: 32)
                Text(state.accessibilityLabel.replacingOccurrences(of: "Phaneros — ", with: ""))
                    .font(.system(size: 10))
                    .foregroundStyle(Palette.textQuaternary)
            }
            .frame(width: 76)
        }
    }
    .padding(30)
    .background(Palette.card)
}
