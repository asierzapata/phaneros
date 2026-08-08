# How to: Release a New Version

This guide covers cutting a new `phaneros`/`phanerosd` release and getting it onto
a machine via Homebrew.

## Prerequisites

- Push access to `asierzapata/phaneros`.
- The `HOMEBREW_TAP_TOKEN` repo secret must be set on `asierzapata/phaneros`
  (a fine-grained PAT scoped to `asierzapata/homebrew-phaneros` with
  **Contents: Read and write**). Without it, the formula bump step fails and
  falls back to a manual step — see [If the formula bump fails](#if-the-formula-bump-fails).

## Cut a release

1. Cut a release using `cargo release`:
   ```bash
   cargo release patch --execute # or minor / major
   ```
   Or manually bump `version` in `Cargo.toml` / workspace crates, commit, tag, and push:
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   The tag must point at a commit already on `main` — pushing the tag is what triggers everything below.
4. This triggers `.github/workflows/release.yml`, which:
   - builds `phaneros` + `phanerosd` for `x86_64-apple-darwin`,
     `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, and `aarch64-unknown-linux-gnu`,
   - publishes a GitHub Release with `phaneros-<target>.tar.gz` +
     `.sha256` for each,
   - bumps `Formula/phaneros.rb` in `asierzapata/homebrew-phaneros` with the
     new version, download URLs, and checksums (for macOS and Linux), and pushes that commit
     directly to the tap repo's default branch.
5. Watch the run: `gh run watch --workflow=release.yml`, or check
   `gh run list --workflow=release.yml`.

Once the tap commit lands, the release is live — no tap-repo release/tag is
needed, Homebrew reads the formula straight off the default branch.

## Get the release onto a machine

```
brew update
brew upgrade asierzapata/phaneros/phaneros
```

`brew update` is required, not optional: `brew upgrade` alone doesn't always
re-pull tap repos (Homebrew throttles automatic tap refreshes), so it can
silently keep offering the previous version.

Verify:

```
phaneros --version
phanerosd --version
```

If the daemon was already running, the new binary won't take effect until it
restarts. Reinstall/restart the login item:

```
phaneros daemon install
```

(safe to re-run — it self-heals a stale or missing launchd registration.)

## If the formula bump fails

If the `update-tap` job fails (expired/missing `HOMEBREW_TAP_TOKEN`), update
`Formula/phaneros.rb` in `asierzapata/homebrew-phaneros` by hand:

1. Download the `.sha256` files from the GitHub Release for macOS and Linux.
2. In `Formula/phaneros.rb`, bump `version` and update the `url`/`sha256`
   pairs under `on_macos` and `on_linux` with the new release's archive URLs and checksums.
3. Commit and push directly to the tap repo's default branch.

## Redeploying `phaneros-store`

`phaneros-store` (the server) is released independently via
`.github/workflows/docker-publish.yml`, which fires automatically on any push
to `main` touching `crates/phaneros-store/**` or `crates/phaneros-sync/**`
(no tag needed). It publishes `ghcr.io/asierzapata/phaneros-store:latest`.
To deploy a new image to the server:

```
docker compose pull
docker compose up -d
```

run wherever `deploy/docker-compose.yml` lives on the host.
