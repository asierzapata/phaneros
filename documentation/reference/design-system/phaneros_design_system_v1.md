# Phaneros Design System (v1.0) Specification

> **Official Design System Specification for Phaneros Desktop & Web UI**
> Encoded into project repository at [`documentation/reference/design-system/design-system.md`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-system/design-system.md).

---

## Visual Reference

````carousel
![State 1: Up to Date with Clean Basenames & Extension Badges](/Users/asierzapata/.gemini/antigravity-cli/brain/c2bfc212-e012-47ff-8e58-96a15cc5d28b/phaneros_tray_clean_basenames_1785840426310.jpg)
<!-- slide -->
![State 2: File Conflict Resolution State](/Users/asierzapata/.gemini/antigravity-cli/brain/c2bfc212-e012-47ff-8e58-96a15cc5d28b/phaneros_tray_conflict_state_1785840283698.jpg)
````

---

## 1. Core Principles

1. **Tactile Paper Substrate:** All interface popups and windows employ a continuous paper background with a subtle micro dot-grid texture (`radial-gradient(#cbd5e1 0.85px, transparent 0.85px)`).
2. **Editorial Typographic Contrast:** `Merriweather` (editorial serif for titles & health headings) paired with `Inter` (sans for component controls & row data) and `JetBrains Mono` (monospaced for metrics, file extensions, and timestamps).
3. **Card Elevation & Depth:** Cards float over the dotted paper canvas using multi-layered soft drop shadows (`box-shadow: 0 12px 28px -4px rgba(0, 0, 0, 0.06)`).
4. **1-Second Glancability:** Instant system health diagnosis via green tick badges (`✓`), active transfer rings (`68%`), or amber warning cards (`!`).

---

## 2. Token Architecture (Shadcn OKLCH)

```css
:root {
  /* Light Mode Palette */
  --background: oklch(0.99 0.002 240);
  --background-paper-texture: radial-gradient(#cbd5e1 0.85px, transparent 0.85px);
  --foreground: oklch(0.148 0.004 228.8); /* Deep Slate */
  --card: oklch(1 0 0); /* Pure White Card Surface */
  --card-foreground: oklch(0.148 0.004 228.8);
  --primary: oklch(0.5 0.134 242.749); /* Cobalt Blue */
  --primary-foreground: oklch(0.977 0.013 236.62);
  --secondary: oklch(0.967 0.001 286.375);
  --secondary-foreground: oklch(0.21 0.006 285.885);
  --muted: oklch(0.963 0.002 197.1);
  --muted-foreground: oklch(0.52 0.02 220);
  --border: oklch(0.92 0.005 214.3);
  --input: oklch(0.925 0.005 214.3);
  --ring: oklch(0.723 0.014 214.4);
  --radius: 0.75rem; /* 12px */

  /* Semantic State Colors */
  --emerald-green: #059669;
  --emerald-bg: #ecfdf5;
  --emerald-border: #a7f3d0;

  --amber-gold: #d97706;
  --amber-bg: #fffbeb;
  --amber-border: #fde68a;

  /* Card Elevation Drop Shadows */
  --card-shadow: 0 12px 28px -4px rgba(0, 0, 0, 0.06), 0 4px 10px -2px rgba(0, 0, 0, 0.03);
  --card-shadow-hover: 0 16px 36px -4px rgba(0, 0, 0, 0.1), 0 6px 14px -2px rgba(0, 0, 0, 0.05);
}
```

---

## 3. Typography Scale Matrix

| Role | Font Family | Size / Weight | Case / Tracking | Example Usage |
| :--- | :--- | :--- | :--- | :--- |
| **Brand Wordmark** | Merriweather | 1.45rem (23px) / 700 | Normal (`-0.02em`) | `Phaneros` in Header |
| **Hero Health Heading**| Merriweather | 1.35rem (21.6px) / 700 | Normal | `Up to Date`, `1 File Conflict` |
| **Section Label** | Inter | 0.72rem (11.5px) / 700 | Uppercase (`0.09em`) | `CONFIGURED DRIVES` |
| **Drive Name** | Inter | 0.92rem (14.7px) / 700 | Normal | `default`, `code-vault` |
| **File Activity Item** | Inter | 0.78rem (12.5px) / 500 | Normal | `sync-protocol`, `syncer` |
| **Extension Badge** | JetBrains Mono | 0.65rem (10.4px) / 700 | Uppercase (`0.05em`) | `MD`, `RS`, `DB` |
| **Timestamp / Metrics**| JetBrains Mono | 0.68rem (10.8px) / 500 | Normal | `2m ago`, `42.8 / 100 GB` |

---

## 4. Prototype & Interactive Files

- 📄 **Repository Spec Document:** [`documentation/reference/design-system/design-system.md`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-system/design-system.md)
- 🖼️ **Refined Tray Design Specification:** [`documentation/reference/design-system/phaneros_refined_tray_design.md`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-system/phaneros_refined_tray_design.md)
- 🖥️ **Interactive Tray Component Reference:** [`phaneros_tray_v3_refined.html`](file:///Users/asierzapata/Documents/Projects/phaneros/phaneros_tray_v3_refined.html)
