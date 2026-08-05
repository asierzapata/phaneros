//! Library surface for `phaneros-daemon`, exposed alongside the `phanerosd`
//! binary so other crates (the desktop app, a future CLI subcommand) can
//! reuse daemon-specific, platform-specific concerns like OS service
//! registration without duplicating them.

#[cfg(target_os = "macos")]
pub mod launchd;
