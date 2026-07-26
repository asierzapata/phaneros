//
//  Snapshots.swift
//  phaneros
//
//  A development-only harness: launch with PHANEROS_SNAPSHOT=<dir> and the app renders
//  every screen to PNG and exits. It exists so the interface can be checked by looking
//  at it, in light and dark, without a human having to walk each state by hand.
//
//  Not compiled into a release build.
//

#if DEBUG
    import ComposableArchitecture
    import SwiftUI

    @MainActor
    enum Snapshots {
        /// The app is sandboxed, so snapshots go to the container's tmp rather than to an
        /// arbitrary path handed in on the command line.
        static var requestedDirectory: URL? {
            guard ProcessInfo.processInfo.environment["PHANEROS_SNAPSHOT"] != nil else {
                return nil
            }
            return URL.temporaryDirectory.appending(path: "phaneros-snapshots")
        }

        static func run(into directory: URL) {
            try? FileManager.default.createDirectory(
                at: directory, withIntermediateDirectories: true)

            for scheme in [ColorScheme.light, .dark] {
                let suffix = scheme == .light ? "light" : "dark"
                let store = fixture()

                render("marks-\(suffix)", scheme: scheme, size: CGSize(width: 520, height: 110)) {
                    HStack(spacing: 18) {
                        ForEach(
                            [MarkState.upToDate, .syncing, .paused, .attention, .offline, .empty],
                            id: \.self
                        ) { state in
                            VStack(spacing: 10) {
                                PhanerosMark(state: state, size: 30, animated: false)
                                Text(
                                    state.accessibilityLabel.replacingOccurrences(
                                        of: "Phaneros — ", with: "")
                                )
                                .font(.system(size: 9))
                                .foregroundStyle(Palette.textQuaternary)
                                .multilineTextAlignment(.center)
                            }
                            .frame(width: 74)
                        }
                    }
                    .padding(20)
                    .background(Palette.card)
                }

                render("glance-\(suffix)", scheme: scheme, size: CGSize(width: 340, height: 460)) {
                    GlanceView(
                        store: store.scope(
                            state: { GlanceFeature.State(from: $0) },
                            action: { .glance($0) }
                        )
                    )
                }

                // `ImageRenderer` can't rasterize NavigationSplitView (it's AppKit-backed),
                // so the window is composed here from the same two views the real scene uses.
                render("window-\(suffix)", scheme: scheme, size: CGSize(width: 980, height: 600)) {
                    HStack(spacing: 0) {
                        DriveSidebar(store: store)
                            .frame(width: 250)
                            .background(Palette.selection.opacity(0.3))
                        Rectangle().fill(Palette.hairline).frame(width: 1)
                        if let driveDetailStore = store.scope(
                            state: \.driveDetail, action: \.driveDetail
                        ) {
                            DriveDetailView(store: driveDetailStore, catchUp: store.state.catchUp)
                        }
                    }
                }

                render("empty-\(suffix)", scheme: scheme, size: CGSize(width: 760, height: 460)) {
                    EmptyStateView(store: fixture(empty: true))
                }

                render("detail-\(suffix)", scheme: scheme, size: CGSize(width: 700, height: 460)) {
                    if let driveDetailStore = store.scope(
                        state: \.driveDetail, action: \.driveDetail
                    ) {
                        DriveDetailView(store: driveDetailStore, catchUp: store.state.catchUp)
                    }
                }

                render("firstsync-\(suffix)", scheme: scheme, size: CGSize(width: 700, height: 340))
                {
                    FirstSyncView(
                        drive: store.state.drives[0],
                        progress: FirstSyncProgress(
                            filesDone: 3800, filesTotal: 10000, estimatedRemaining: 720)
                    )
                }

                render("offline-\(suffix)", scheme: scheme, size: CGSize(width: 700, height: 300)) {
                    StoreUnreachableView(host: "phaneros.mattstudio.dev")
                }

                render("reconnect-\(suffix)", scheme: scheme, size: CGSize(width: 700, height: 340))
                {
                    ReconnectView(store: store, host: "phaneros.mattstudio.dev")
                }

                render(
                    "firstrun-1-folder-\(suffix)", scheme: scheme,
                    size: CGSize(width: 480, height: 560)
                ) {
                    let firstRunStore = Store(
                        initialState: FirstRunFeature.State(step: .folder)
                    ) {
                        FirstRunFeature()
                    }
                    FirstRunView(store: firstRunStore)
                }

                render(
                    "firstrun-2-store-\(suffix)", scheme: scheme,
                    size: CGSize(width: 480, height: 600)
                ) {
                    let firstRunStore = Store(
                        initialState: FirstRunFeature.State(
                            driveName: "Notes",
                            step: .store,
                            storeHost: "phaneros.mattstudio.dev"
                        )
                    ) {
                        FirstRunFeature()
                    }
                    FirstRunView(store: firstRunStore)
                }

                render(
                    "firstrun-3-confirm-\(suffix)", scheme: scheme,
                    size: CGSize(width: 480, height: 480)
                ) {
                    let firstRunStore = Store(
                        initialState: FirstRunFeature.State(
                            driveName: "Notes",
                            folder: URL(fileURLWithPath: "/Users/matt/Notes"),
                            step: .confirm,
                            storeHost: "phaneros.mattstudio.dev"
                        )
                    ) {
                        FirstRunFeature()
                    }
                    FirstRunView(store: firstRunStore)
                }

                render("adddrive-\(suffix)", scheme: scheme, size: CGSize(width: 440, height: 360))
                {
                    let addDriveStore = Store(
                        initialState: AddDriveFeature.State(
                            folder: URL(fileURLWithPath: "/Users/matt/Notes"),
                            name: "Notes",
                            isPresented: true
                        )
                    ) {
                        AddDriveFeature()
                    }
                    AddDriveSheet(store: addDriveStore)
                }

                render(
                    "removedrive-\(suffix)", scheme: scheme, size: CGSize(width: 440, height: 300)
                ) {
                    let removeDriveStore = Store(
                        initialState: RemoveDriveFeature.State(
                            drive: Drive(
                                id: "snapshot",
                                name: "Work",
                                path: URL(fileURLWithPath: "/Users/matt/Work"),
                                status: .upToDate(at: .now)
                            ),
                            isPresented: true
                        )
                    ) {
                        RemoveDriveFeature()
                    }
                    RemoveDriveSheet(store: removeDriveStore)
                }

                // Likewise TabView — the three panes are rendered directly.
                let settingsStore = Store(
                    initialState: SettingsFeature.State(from: store.state)
                ) {
                    SettingsFeature()
                }
                render(
                    "settings-general-\(suffix)", scheme: scheme,
                    size: CGSize(width: 520, height: 260)
                ) {
                    GeneralSettings(store: settingsStore)
                }

                render(
                    "settings-notifications-\(suffix)", scheme: scheme,
                    size: CGSize(width: 520, height: 260)
                ) {
                    NotificationSettings(store: settingsStore)
                }

                render(
                    "settings-advanced-\(suffix)", scheme: scheme,
                    size: CGSize(width: 520, height: 260)
                ) {
                    AdvancedSettings(store: settingsStore)
                }
            }

            log("snapshots written to \(directory.path)")
        }

        private static func log(_ message: String) {
            FileHandle.standardError.write(Data("[snapshot] \(message)\n".utf8))
        }

        private static func fixture(empty: Bool = false) -> StoreOf<AppFeature> {
            let store = Store(
                initialState: AppFeature.State(),
                reducer: { AppFeature(client: DemoClient()) }
            )
            if !empty {
                // Give the listener a beat to deliver the demo drives.
                store.send(.startListening)
                RunLoop.current.run(until: .now.addingTimeInterval(0.35))
            }
            return store
        }

        private static func render(
            _ name: String,
            scheme: ColorScheme,
            size: CGSize,
            @ViewBuilder content: () -> some View
        ) {
            // Palette colours are dynamic NSColors, so they resolve against the *drawing*
            // appearance rather than SwiftUI's colorScheme environment. Both have to be set
            // or the dark renders come out identical to the light ones.
            let appearance = NSAppearance(named: scheme == .dark ? .darkAqua : .aqua)

            let renderer = ImageRenderer(
                // colorScheme has to wrap the background too, or the backdrop resolves
                // against the ambient appearance while the content resolves against this one.
                content: content()
                    .environment(\.isSnapshotting, true)
                    .frame(width: size.width, height: size.height)
                    .background(Palette.card)
                    .environment(\.colorScheme, scheme)
            )
            renderer.scale = 2

            var rendered: NSImage?
            appearance?.performAsCurrentDrawingAppearance { rendered = renderer.nsImage }

            guard
                let image = rendered,
                let tiff = image.tiffRepresentation,
                let rep = NSBitmapImageRep(data: tiff),
                let png = rep.representation(using: .png, properties: [:]),
                let directory = requestedDirectory
            else {
                log("FAILED to render \(name)")
                return
            }

            do {
                try png.write(to: directory.appending(path: "\(name).png"))
            } catch {
                log("FAILED to write \(name): \(error)")
            }
        }
    }
#endif
