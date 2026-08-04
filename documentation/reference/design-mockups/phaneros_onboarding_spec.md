# Phaneros — Arc-Inspired "Unboxing" Onboarding Specification (v3.0)

> **Design Alignment:** Arc Browser-inspired "unboxing" narrative, interactive theme & color picker, 3D interactive **Phaneros Vault Member Card**, progressive line progress track bar, ambient background glow, zero-emoji policy, and seamless storage target selection.

---

## 🖼️ Interactive Onboarding Prototype

🔗 **[Open Arc-Inspired Unboxing Prototype](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-mockups/phaneros_onboarding_v1.html)**

*Open in your web browser to experience the Arc Browser-style "unboxing" journey, test the **interactive accent theme picker**, configure storage targets, inspect the **personalized 3D Vault Member Card**, and launch into the Control Center.*

---

## 🎨 Arc-Inspired "Unboxing" Journey Architecture

Unlike traditional dull configuration wizards, Phaneros implements an **Arc-style "unboxing" experience** prioritizing emotional engagement, tactile feedback, and narrative delight:

```
 ┌─────────────────────────────────────────────────────────────────────────────────┐
 │ [Progress Track Bar ========================>                                 ] │
 │ Phaneros                                                         Unboxing 1 of 5│
 ├─────────────────────────────────────────────────────────────────────────────────┤
 │                                                                                 │
 │ 1. Meet Phaneros — Animated Sync Engine Micro-Diagram                           │
 │ 2. Workspace Vibe — Interactive Color & Theme Selector (Cobalt, Emerald, Amber) │
 │ 3. Storage Target — Phaneros Cloud ($4/mo Trial) vs Self-Hosted Server Endpoint │
 │ 4. Synced Folders — Configure local disk folder vaults                          │
 │ 5. Phaneros Vault Member Card — 3D Personalized Interactive Identity Card       │
 │                                                                                 │
 └─────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Meet Phaneros (Step 1)
- Narrative intro to local-first file synchronization.
- **Interactive Micro-Diagram:** Animated vector beam flow connecting Local Disk node, Blake3 Engine, and E2EE Vault.

### 2. Workspace Vibe & Color Customization (Step 2 — Arc Signature)
- Interactive theme palette selector:
  - **Cobalt Blue** (Default OKLCH primary)
  - **Emerald Green** (High-trust emerald green)
  - **Amber Gold** (Warm editorial gold accent)
  - **Slate Gray** (Minimal dark slate)
- Live color preview updating UI CSS custom properties (`--accent`, `--accent-light`, `--accent-border`) in real-time as the user picks a vibe!

### 3. Storage Target Selection (Step 3)
- **Phaneros Cloud (SaaS):** Zero-configuration managed cloud storage with 7-Day Free Trial ($4/mo).
- **Self-Hosted Server:** Open source private instance with custom endpoint URL.

### 4. Synced Folders Setup (Step 4)
- Local folder drive selection (`default` ➔ `~/Documents/PhanerosSync`, `code-vault` ➔ `~/Projects/Vault`).

### 5. Personalized Phaneros Vault Member Card (Step 5 — Arc Signature)
- Generates an interactive 3D perspective **Phaneros Vault Member Card**:
  - Displays machine owner identity (`asierzapata @ Mac-Studio`), Vault Badge ID (`#VAULT-0842`), storage destination, encryption status, and active drive count.
  - Interactive 3D tilt & hover physics on cursor movement.
- **Primary CTA:** "Enter Control Center ↗" (launches `phaneros_main_app_v1.html`).

---

## 📄 Reference Files

- 🖥️ **[Interactive Arc-Inspired Onboarding: `phaneros_onboarding_v1.html`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-mockups/phaneros_onboarding_v1.html)**
- 🖥️ **[Main App Control Center Prototype: `phaneros_main_app_v1.html`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-mockups/phaneros_main_app_v1.html)**
- 📄 **[Main App Specification: `phaneros_main_app_spec.md`](file:///Users/asierzapata/Documents/Projects/phaneros/documentation/reference/design-mockups/phaneros_main_app_spec.md)**
