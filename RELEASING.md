# Releasing the phaneros CLI

1. Bump `version` in `crates/phaneros-cli/Cargo.toml` and `crates/phaneros-daemon/Cargo.toml` if needed.
2. Commit, then tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. This triggers `.github/workflows/release.yml`, which builds `phaneros` + `phanerosd`
   for `x86_64-apple-darwin` and `aarch64-apple-darwin`, and publishes a GitHub Release
   with `phaneros-<target>.tar.gz` + `.sha256` for each.
4. In the `asierzapata/homebrew-phaneros` tap repo, update `Formula/phaneros.rb`:
   - Bump `version`.
   - Replace the two `url`/`sha256` pairs with the new release's archive URLs and the
     contents of the matching `.sha256` files.
5. Commit and push the formula update. `brew install asierzapata/phaneros/phaneros`
   (or `brew upgrade` for existing installs) picks it up immediately — no tap-repo
   release/tag needed, Homebrew reads the formula directly off the default branch.

## Verifying a release

```
brew tap asierzapata/phaneros
brew install phaneros
phaneros setup
```

Confirm both `phaneros` and `phanerosd` landed in `$(brew --prefix)/bin` and that
`phaneros setup` runs without needing `$PATH` changes.
