//
//  phanerosApp.swift
//  phaneros
//
//  Job #1: answer "am I fine?" without being opened.
//
//  The menu bar is the primary interface and most users will never look past it, so
//  the app launches as an accessory — no Dock icon, no window — and only becomes a
//  regular app once someone actually asks for the window. A good week is one where
//  nobody gets past the icon.
//

import ComposableArchitecture
import SwiftUI

@main
struct PhanerosApp: App {
    @State private var store = Store(
        initialState: AppFeature.State(),
        reducer: { AppFeature(client: DemoClient()) }
    )
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate

    var body: some Scene {
        MenuBarExtra {
            GlanceView(
                store: store.scope(
                    state: { GlanceFeature.State(from: $0) },
                    action: { .glance($0) }
                )
            )
        } label: {
            // Rendered to a non-template image so the five states keep their colour.
            Image(
                nsImage: PhanerosMark.menuBarImage(
                    for: store.state.overallMark,
                    appearance: NSApp.effectiveAppearance
                ))
        }
        .menuBarExtraStyle(.window)

        Window("Phaneros", id: "main") {
            MainWindowView(store: store)
                .onAppear { NSApp.setActivationPolicy(.regular) }
                .onDisappear { NSApp.setActivationPolicy(.accessory) }
        }
        .defaultSize(width: 980, height: 640)
        // The menu bar is the product. The window opens when asked for, not at launch.
        .defaultLaunchBehavior(.suppressed)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Add a Drive…") { store.send(.addDriveTapped) }
                    .keyboardShortcut("n")
            }
        }

        Settings {
            SettingsView(
                store: store.scope(
                    state: { SettingsFeature.State(from: $0) },
                    action: { .settings($0) }
                )
            )
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)

        #if DEBUG
            if let directory = Snapshots.requestedDirectory {
                Snapshots.run(into: directory)
                NSApp.terminate(nil)
                return
            }
        #endif

        Notifier.requestAuthorizationIfNeeded()
    }

    /// Closing the window shouldn't quit — the whole point is that it keeps working
    /// while nobody is looking at it.
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }
}
