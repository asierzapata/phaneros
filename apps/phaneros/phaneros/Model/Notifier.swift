//
//  Notifier.swift
//  phaneros
//
//  System notifications, and the discipline about when not to post one.
//
//  From the design: notifications fire for a conflict, a connection that needs
//  reconnecting, and a first big sync finishing. They never fire for routine syncing,
//  brief offline blips, waking up and catching up, or pausing and resuming — the user
//  did that last one themselves. Everything else lives in the icon and the popover,
//  unread until someone looks.
//

import UserNotifications

enum Notifier {
    static func requestAuthorizationIfNeeded() {
        UNUserNotificationCenter.current()
            .requestAuthorization(options: [.alert, .sound]) { _, _ in }
    }

    static func conflict(_ conflict: Conflict, enabled: Bool) {
        post(
            id: "conflict-\(conflict.id)",
            title: conflict.title,
            body: conflict.body,
            enabled: enabled
        )
    }

    static func tokenRejected(host: String, enabled: Bool) {
        post(
            id: "reconnect-\(host)",
            title: "\(host) needs you to reconnect",
            body: "Nothing new will sync until then. Your files are untouched.",
            enabled: enabled
        )
    }

    static func firstSyncFinished(drive: String, fileCount: Int, enabled: Bool) {
        post(
            id: "first-sync-\(drive)",
            title: "\(drive) is all here",
            body: "\(fileCount.formatted(.number)) files brought over. It'll just keep up from here.",
            enabled: enabled
        )
    }

    private static func post(id: String, title: String, body: String, enabled: Bool) {
        guard enabled else { return }
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        UNUserNotificationCenter.current().add(
            UNNotificationRequest(identifier: id, content: content, trigger: nil)
        )
    }
}
