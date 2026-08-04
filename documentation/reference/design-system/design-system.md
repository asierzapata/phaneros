# Phaneros Design System Specification (v1.0)

> **Core Identity:** A modern, minimalist, high-trust Silicon Valley design system pairing tactile paper substrates with editorial serif typography and OKLCH color tokens.

---

## 1. Design Principles

1. **Tactile Paper Substrate:** All views use a continuous warm paper background texture with a subtle micro dot-grid background (`radial-gradient(#cbd5e1 0.85px, transparent 0.85px)`). Header regions flow seamlessly without blocky solid background banners.
2. **Editorial Typographic Hierarchy:** Pair `Merriweather` (editorial serif for brand title & health headings) with `Inter` (precision sans for interface controls & drive names) and `JetBrains Mono` (monospaced for metrics, file extension badges, and timestamps).
3. **Floating Card Elevation:** Interactive cards float over the dot-grid canvas with soft, multi-layered drop shadows (`box-shadow: 0 12px 28px -4px rgba(0, 0, 0, 0.06), 0 4px 10px -2px rgba(0, 0, 0, 0.03)`).
4. **Instant 1-Second Glancability:** Answer system health ("Is everything syncing well?") in under 0.5s using clear, color-coded visual badges (`✓` Green tick for Up to Date, `68%` Progress Ring for active transfers, `!` Amber Warning for conflicts).

---

## 2. Color Palette & Token System (OKLCH)

Phaneros uses standard Shadcn OKLCH color tokens supporting seamless Light and Dark mode transitions:

### CSS Custom Properties
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

body.dark {
  /* Dark Mode Palette */
  --background: oklch(0.148 0.004 228.8);
  --background-paper-texture: radial-gradient(rgba(255,255,255,0.09) 0.85px, transparent 0.85px);
  --foreground: oklch(0.987 0.002 197.1);
  --card: oklch(0.218 0.008 223.9);
  --card-foreground: oklch(0.987 0.002 197.1);
  --secondary: oklch(0.274 0.006 286.033);
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.275 0.011 216.9);
  --muted-foreground: oklch(0.723 0.014 214.4);
  --border: oklch(1 0 0 / 10%);
  --input: oklch(1 0 0 / 15%);

  --emerald-green: #34d399;
  --emerald-bg: rgba(16, 185, 129, 0.12);
  --emerald-border: rgba(16, 185, 129, 0.3);

  --amber-gold: #fbbf24;
  --amber-bg: rgba(245, 158, 11, 0.12);
  --amber-border: rgba(245, 158, 11, 0.3);

  --card-shadow: 0 14px 30px rgba(0, 0, 0, 0.4);
  --card-shadow-hover: 0 18px 40px rgba(0, 0, 0, 0.6);
}
```

---

## 3. Typography Scale & Rules

```
Serif (Merriweather):     Titles, Brand Wordmarks, Hero Health Headings
Sans (Inter):             Card Headings, Drive Names, Component Controls, Body Text
Mono (JetBrains Mono):    File Extension Badges, Storage Quota Ratios, Timestamps
```

| Element | Font Family | Size / Weight | Case / Tracking | Example Usage |
| :--- | :--- | :--- | :--- | :--- |
| **Brand Wordmark** | Merriweather | 1.45rem (23px) / 700 | Normal (`-0.02em`) | `Phaneros` in Header |
| **Hero Health Heading**| Merriweather | 1.35rem (21.6px) / 700 | Normal | `Up to Date`, `1 File Conflict` |
| **Section Label** | Inter | 0.72rem (11.5px) / 700 | Uppercase (`0.09em`) | `CONFIGURED DRIVES` |
| **Drive Name** | Inter | 0.92rem (14.7px) / 700 | Normal | `default`, `code-vault` |
| **File Activity Item** | Inter | 0.78rem (12.5px) / 500 | Normal | `sync-protocol`, `syncer` |
| **Extension Badge** | JetBrains Mono | 0.65rem (10.4px) / 700 | Uppercase (`0.05em`) | `MD`, `RS`, `DB` |
| **Timestamp / Metrics**| JetBrains Mono | 0.68rem (10.8px) / 500 | Normal | `2m ago`, `42.8 / 100 GB` |

---

## 4. Core Component Specs

### A. Tray Popup Container
- Width: `380px`
- Border Radius: `var(--radius)` (12px)
- Padding: `16px` bottom
- Background: Dotted paper background continuous through header.

### B. Top Header Bar
- Title: `Merriweather` bold wordmark (`Phaneros`).
- Action Icons (Right): 32px floating card icon buttons:
  - `↗` **Launch Full App:** Opens Desktop Control Center.
  - `⚙` **Preferences:** Opens settings modal.
- No solid background card, no status pill.

### C. Hero Health Card
- Floating card (`var(--card)`) with `var(--card-shadow)`.
- Green Tick Badge (`✓`): 38px circular badge with `#059669` green background tint.
- Syncing State: Conic progress ring (`68%`) with live transfer rate (`1.4 MB/s`).

### D. Drive Stack Cards
- One floating card per configured drive (`default`, `code-vault`).
- Includes mini ring gauge chart (`42%`), capacity text (`42.8 / 100 GB`), and status timestamp.
- **Click Behavior:** Clicking any drive card triggers Finder file explorer (`~/Documents/PhanerosSync`).

### E. File Activity Stream
- Filenames displayed as **basenames only** (no path, no extension).
- Paired with monospaced extension pill (`MD`, `RS`, `DB`) without emojis.

---

## 5. Reference Implementations

- 📄 **Interactive Reference Component:** [`phaneros_tray_v3_refined.html`](file:///Users/asierzapata/Documents/Projects/phaneros/phaneros_tray_v3_refined.html)
- 🖼️ **Refined Tray Design Specification:** [`phaneros_refined_tray_design.md`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-system/phaneros_refined_tray_design.md)
- 🖼️ **Design System v1.0 Overview:** [`phaneros_design_system_v1.md`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-system/phaneros_design_system_v1.md)
