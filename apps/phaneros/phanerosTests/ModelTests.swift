//
//  ModelTests.swift
//  phanerosTests
//
//  Tests for computed properties on domain models.
//  These verify existing behavior and serve as a safety net during migration.
//

import Foundation
import Testing

@testable import phaneros

// MARK: - DriveStatus

struct DriveStatusTests {

    @Test func markUpToDate() {
        let status = DriveStatus.upToDate(at: .now)
        #expect(status.mark == .upToDate)
    }

    @Test func markWorking() {
        #expect(DriveStatus.working.mark == .syncing)
    }

    @Test func markPaused() {
        #expect(DriveStatus.paused.mark == .paused)
    }

    @Test func markNeedsAttention() {
        let status = DriveStatus.needsAttention(reason: "conflict")
        #expect(status.mark == .attention)
    }

    @Test func markOffline() {
        #expect(DriveStatus.offline.mark == .offline)
    }

    @Test func labelUpToDate() {
        let date = Date(timeIntervalSinceNow: -30)
        let status = DriveStatus.upToDate(at: date)
        #expect(status.label.contains("Synced"))
        #expect(status.label.contains("just now"))
    }

    @Test func labelWorking() {
        #expect(DriveStatus.working.label == "Syncing…")
    }

    @Test func labelPaused() {
        #expect(DriveStatus.paused.label == "Paused")
    }

    @Test func labelNeedsAttention() {
        #expect(DriveStatus.needsAttention(reason: "conflict").label == "Needs a look")
    }

    @Test func labelOffline() {
        #expect(DriveStatus.offline.label == "Can't reach the store")
    }

    @Test func shortLabelUpToDate() {
        #expect(DriveStatus.upToDate(at: .now).shortLabel == "Up to date")
    }

    @Test func shortLabelWorking() {
        #expect(DriveStatus.working.shortLabel == "Syncing…")
    }

    @Test func shortLabelPaused() {
        #expect(DriveStatus.paused.shortLabel == "Paused")
    }

    @Test func shortLabelNeedsAttention() {
        #expect(DriveStatus.needsAttention(reason: "conflict").shortLabel == "Needs attention")
    }

    @Test func shortLabelOffline() {
        #expect(DriveStatus.offline.shortLabel == "Can't reach the store")
    }
}

// MARK: - Conflict

struct ConflictTests {

    private func makeConflict(kind: Conflict.Kind) -> Conflict {
        Conflict(
            fileName: "Budget.xlsx",
            keptCopyName: "Budget (Kept Copy).xlsx",
            otherDevice: "MacBook Air",
            kind: kind,
            at: .now,
            fileURL: URL(fileURLWithPath: "/tmp/Budget.xlsx")
        )
    }

    @Test func titleBothEdited() {
        let conflict = makeConflict(kind: .bothEdited)
        #expect(conflict.title == "Kept both versions of Budget.xlsx")
    }

    @Test func titleEditedAndDeleted() {
        let conflict = makeConflict(kind: .editedAndDeleted)
        #expect(conflict.title == "Kept Budget.xlsx, which another device deleted")
    }

    @Test func bodyBothEdited() {
        let conflict = makeConflict(kind: .bothEdited)
        let body = conflict.body
        #expect(body.contains("Edited on two devices while apart"))
        #expect(body.contains("nothing was lost"))
        #expect(body.contains("Budget (Kept Copy).xlsx"))
    }

    @Test func bodyEditedAndDeleted() {
        let conflict = makeConflict(kind: .editedAndDeleted)
        let body = conflict.body
        #expect(body.contains("MacBook Air deleted it"))
        #expect(body.contains("nothing was lost"))
        #expect(body.contains("Budget (Kept Copy).xlsx"))
    }
}

// MARK: - Drive

struct DriveTests {

    private func makeDrive(path: URL) -> Drive {
        Drive(
            id: "test-drive",
            name: "Notes",
            path: path,
            status: .working
        )
    }

    @Test func displayPathWithTilde() {
        let home = FileManager.default.homeDirectoryForCurrentUser
        let path = home.appendingPathComponent("Documents/Notes")
        let drive = makeDrive(path: path)
        #expect(drive.displayPath == "~/Documents/Notes")
    }

    @Test func displayPathWithoutTilde() {
        let path = URL(fileURLWithPath: "/Volumes/External/Notes")
        let drive = makeDrive(path: path)
        #expect(drive.displayPath == "/Volumes/External/Notes")
    }

    @Test func displayPathHomeDirectoryItself() {
        let home = FileManager.default.homeDirectoryForCurrentUser
        let drive = makeDrive(path: home)
        #expect(drive.displayPath == "~")
    }
}

// MARK: - FirstSyncProgress

struct FirstSyncProgressTests {

    @Test func fractionNormal() {
        let progress = FirstSyncProgress(filesDone: 3000, filesTotal: 10000)
        #expect(progress.fraction == 0.3)
    }

    @Test func fractionComplete() {
        let progress = FirstSyncProgress(filesDone: 10000, filesTotal: 10000)
        #expect(progress.fraction == 1.0)
    }

    @Test func fractionZeroTotal() {
        let progress = FirstSyncProgress(filesDone: 0, filesTotal: 0)
        #expect(progress.fraction == 0)
    }

    @Test func fractionOverComplete() {
        // Should be clamped to 1.0
        let progress = FirstSyncProgress(filesDone: 15000, filesTotal: 10000)
        #expect(progress.fraction == 1.0)
    }

    @Test func captionWithoutRemaining() {
        let progress = FirstSyncProgress(
            filesDone: 3800, filesTotal: 10000, estimatedRemaining: nil)
        let caption = progress.caption
        // Grouping separators follow the run locale, so compare against the same formatter.
        #expect(caption.contains(3800.formatted(.number)))
        #expect(caption.contains(10000.formatted(.number)))
        #expect(!caption.contains("left"))
    }

    @Test func captionWithRemaining() {
        let progress = FirstSyncProgress(
            filesDone: 3800, filesTotal: 10000, estimatedRemaining: 720)
        let caption = progress.caption
        #expect(caption.contains(3800.formatted(.number)))
        #expect(caption.contains(10000.formatted(.number)))
        #expect(caption.contains("12 minutes"))
        #expect(caption.contains("left"))
    }

    @Test func captionWithZeroRemaining() {
        let progress = FirstSyncProgress(filesDone: 5000, filesTotal: 10000, estimatedRemaining: 0)
        let caption = progress.caption
        #expect(!caption.contains("left"))
    }
}

// MARK: - Date.phanerosRelative

struct DateRelativeTests {

    @Test func justNow() {
        let date = Date(timeIntervalSinceNow: -10)
        #expect(date.phanerosRelative == "just now")
    }

    @Test func oneMinuteAgo() {
        let date = Date(timeIntervalSinceNow: -60)
        #expect(date.phanerosRelative == "1 min ago")
    }

    @Test func severalMinutesAgo() {
        let date = Date(timeIntervalSinceNow: -300)  // 5 minutes
        #expect(date.phanerosRelative == "5 min ago")
    }

    @Test func oneHourAgo() {
        let date = Date(timeIntervalSinceNow: -4200)  // 70 minutes
        #expect(date.phanerosRelative == "1 hr ago")
    }

    @Test func severalHoursAgo() {
        let date = Date(timeIntervalSinceNow: -10800)  // 3 hours
        #expect(date.phanerosRelative == "3 hr ago")
    }

    @Test func yesterday() {
        let date = Date(timeIntervalSinceNow: -100000)  // ~27.8 hours
        #expect(date.phanerosRelative == "yesterday")
    }

    @Test func olderThanYesterday() {
        let date = Date(timeIntervalSinceNow: -200000)  // ~55.6 hours
        let result = date.phanerosRelative
        // Should be formatted like "Jul 24"
        #expect(!result.contains("ago"))
        #expect(!result.contains("yesterday"))
        #expect(!result.contains("just now"))
    }
}

// MARK: - TimeInterval.phanerosCoarseDuration

struct TimeIntervalCoarseDurationTests {

    @Test func lessThanMinute() {
        let duration: TimeInterval = 45
        #expect(duration.phanerosCoarseDuration == "a minute")
    }

    @Test func severalMinutes() {
        let duration: TimeInterval = 720  // 12 minutes
        #expect(duration.phanerosCoarseDuration == "12 minutes")
    }

    @Test func oneHour() {
        let duration: TimeInterval = 3600
        #expect(duration.phanerosCoarseDuration == "an hour")
    }

    @Test func severalHours() {
        let duration: TimeInterval = 7200  // 2 hours
        #expect(duration.phanerosCoarseDuration == "2 hours")
    }
}
