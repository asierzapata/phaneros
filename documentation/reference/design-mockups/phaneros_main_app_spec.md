# Phaneros — Main Desktop Control Center Specification (v1.5)

> **Design Alignment:** Seamless transparent top bar with centered Apple HIG Segmented Tab Control, theme toggle control, zero-emoji policy, Diffs.com code comparison engine, and Trees.software drive explorer layout.

---

## 🖼️ Interactive UI Mockup Prototype

🔗 **[Open Updated Main App Control Center Prototype](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-mockups/phaneros_main_app_v1.html)**

*Open in your web browser to test the transparent top bar with centered Apple HIG segmented control tabs, toggle Light/Dark mode, inspect the **Trees.software** drive explorer layout, and test both **Diffs.com-style text diffs** and **digestible binary file metadata comparison**.*

---

## 🍏 Centered Apple HIG Top Header Refinements

1. **Seamless Transparent Substrate:**
   - Completely transparent header background with no border, allowing the tactile paper dot-grid texture to flow continuously through the window.
2. **Centered Segmented Tab Control:**
   - Apple HIG Segmented Control tab bar (`Dashboard`, `Drives & Files`, `Conflicts`, `Activity`, `Settings`) centered in the top window bar.
   - Symmetrically balanced with the brand wordmark (`Phaneros v1.0`) on the left and the theme toggle control on the right.
3. **Clean Action Layout:**
   - Removed top search bar, server connection pill badge, and duplicate sync button from top header for maximum visual clarity.

---

## 🛠️ Module Suite Architecture

```
 ┌─────────────────────────────────────────────────────────────────────────────────┐
 │ Phaneros v1.0         [ Dashboard | Drives & Files | Conflicts (1) | Activity ]          [🌗] │
 └─────────────────────────────────────────────────────────────────────────────────┘
```

### 1. System Dashboard Module
- **Hero Status Banner:** Instant status check (`✓ Everything is Up to Date`) with manual `Sync Now` action button.
- **Telemetry Grid:**
  - **Last Synced:** `2m ago` (Automatic cloud background sync)
  - **Deduplication Ratio:** `1.85×` (Saved 42.8 GB across identical files)
  - **Compression Ratio:** `32%` (Reduced transfer payload size)
  - **Transfer Speed:** `4.2 MB/s` (Peak bandwidth utilization)
- **Configured Storage Drives:** Floating cards displaying quota progress bar, path, last sync time, and folder exploration trigger.

### 2. Drives & Files Explorer (Trees.software Style)
- **Left Column:** Drive selector list (`default`, `code-vault`, `media-archive`).
- **Right Column Top:** Drive metadata summary card displaying Drive Name, Local Path (`~/Documents/PhanerosSync`), Storage Usage (`42.8 / 100 GB`), Last Synced timestamp, Storage Saved ratio, and Server Connection state.
- **Right Column Bottom:** Interactive hierarchical file tree with branch guide lines, file type badges (`MD`, `DB`), item sizes, modification dates, and status pills (`✓ Synced`, `Conflict`).

### 3. Sync Conflicts Workspace (Diffs.com Style)
- **Text & Code Files (`README.md`, `.rs`, `.json`):**
  - Side-by-Side Split View powered by **Diffs.com** word-level diffing algorithms.
  - Highlights specific changed words (`+ added text` in soft emerald tint, `- removed text` in soft rose tint).
  - Clean action toolbar (`Keep My Local File`, `Keep Server Version`, `Save Merged File`).
- **Opaque Binary Files (`database.sqlite`, `.db`, `.zip`):**
  - Digestible metadata comparison matrix comparing File Sizes (`14.2 MB` vs `15.8 MB`), Last Modified Times, Unique Version IDs (`v4-local` vs `v3-server`), and recommended actions.

### 4. Activity Log Module
- Background sync activity history stream recording file changes, conflict triggers, and completion statuses.

### 5. Settings Module
- Central storage connection credentials, server endpoints, background sync toggles, and compression preferences.

---

## 3. Reference Files

- 📄 **Interactive Main App Mockup:** [`documentation/reference/design-mockups/phaneros_main_app_v1.html`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-mockups/phaneros_main_app_v1.html)
- 📄 **Design System v1.0 Overview:** [`documentation/reference/design-system/design-system.md`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-system/design-system.md)
- 📄 **Tray UI Refined Mockup:** [`phaneros_tray_v3_refined.html`](file:///Users/asierzapata/Documents/Projects/phaneros/phaneros_tray_v3_refined.html)
