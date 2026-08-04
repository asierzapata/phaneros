# Phaneros — Refined Tray UI Specification

> **Design Alignment:** Incorporating all 6 points of feedback into a sleek, non-generic, high-trust macOS tray popup interface.

---

## Visual Previews

````carousel
![State 1: Up to Date with Clean Basenames & Extension Badges](/Users/asierzapata/.gemini/antigravity-cli/brain/c2bfc212-e012-47ff-8e58-96a15cc5d28b/phaneros_tray_clean_basenames_1785840426310.jpg)
<!-- slide -->
![State 2: File Conflict Resolution State](/Users/asierzapata/.gemini/antigravity-cli/brain/c2bfc212-e012-47ff-8e58-96a15cc5d28b/phaneros_tray_conflict_state_1785840283698.jpg)
````

---

## Interactive Prototype

🔗 **[Open Refined Tray UI Interactive Prototype](file:///Users/asierzapata/Documents/Projects/phaneros/phaneros_tray_v3_refined.html)**

*Open in your browser to test state transitions (`Up to Date` vs `Syncing` vs `Conflict`), test clicking drive cards to open local paths, toggle the Recent Activity section on/off, and switch between Light/Dark mode.*

---

## Feedback Implementations Breakdown

### 1. Continuous Background Texture
* **What changed:** Removed the solid white header card block. The tactile paper dot-grid background texture (`radial-gradient(#cbd5e1 0.85px, transparent 0.85px)`) now extends continuously through the top header all the way down to the bottom.

### 2. Depth Elevation & Shadows
* **What changed:** Cards float over the dotted canvas with multi-layered soft drop shadows (`box-shadow: 0 12px 28px -4px rgba(0, 0, 0, 0.06)`), creating a tangible physical paper depth feeling.

### 3. Sync Health & Header Cleanup
* **What changed:** 
  - **Up to Date Badge:** Replaced the redundant `100%` ring dial with a crisp green checkmark (`✓`) in an emerald circular badge (`#059669`).
  - **Pill Removal:** Removed the redundant status pill badge from the header.
  - **Dynamic Sync State:** Only shows a progress ring dial (`68%`) when syncing is actively in progress (`1.4 MB/s • ~10s remaining`).

### 4. Per-Drive Cards & Local Finder Launcher
* **What changed:** Removed the drive tab pills. Each configured drive (`default`, `code-vault`) gets a dedicated floating card containing:
  - **Drive Title & Path:** e.g. `default` • `~/Documents/PhanerosSync`.
  - **Circular Storage Ring Gauge:** Displays percentage used (e.g. `42%`) and capacity (`42.8 / 100 GB`).
  - **Dynamic Status:** Shows `Last synced 2m ago` (or active transfer speed when syncing).
  - **Click-to-Open Folder:** **Clicking any drive card directly opens its folder in macOS Finder!** Hovering over a card displays a subtle `Finder ↗` prompt.

### 5. Recent Activity: Extension Badges & Clean Basenames
* **What changed:** 
  - Emojis completely removed.
  - Directory paths and `.ext` suffixes removed from filenames (showing basenames `sync-protocol`, `syncer`, `phaneros`).
  - Monospaced extension badge pills (`MD`, `RS`, `DB`) rendered in `JetBrains Mono`.

### 6. Clean Navigation & Toolbar Elimination
* **What changed:** Removed the bottom action toolbar ("Open Folder", "Reconcile Vault") completely.
* **Top Header Controls:** Placed two clean icon buttons in the top-right header:
  1. `↗` **Launch Full App:** Opens the main Phaneros Desktop Control Center window.
  2. `⚙` **Preferences:** Opens app configuration & settings.
