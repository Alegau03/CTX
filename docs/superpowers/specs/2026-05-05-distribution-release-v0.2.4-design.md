# CTX v0.2.4 Distribution And Release Design

## Goal

Publish CTX `v0.2.4` as a real public release across the supported distribution channels with a clean release flow, coherent versioning, verified artifacts, and synchronized documentation.

The release must ship:

- GitHub Release artifacts as the source of truth
- `cargo install ctx` through a published crate
- `npm i -g @alegau/ctx-bin` through a published npm package
- `brew tap Alegau03/ctx && brew install ctx` through the public Homebrew tap
- `ctx update` and `ctx update --check` documented consistently across install surfaces

This release intentionally excludes the final README GIFs and screenshots, which will be recorded after the release mechanics are finished and verified.

## Scope

This design covers:

- release preparation on `dev`
- version bump to `0.2.4`
- release build and verification
- merge `dev -> main`
- tag creation and GitHub Release publishing
- crate publication to `crates.io`
- npm publication of `@alegau/ctx-bin`
- Homebrew tap update and publication
- install/update documentation synchronization
- public-channel smoke verification

This design does not cover:

- README media asset production
- launch post copywriting
- post-release marketing assets

## Release Architecture

The release should be driven from the repository state, not from ad hoc manual uploads.

The flow for `v0.2.4` is:

1. finish and verify the release changes on `dev`
2. bump all release-visible version references to `0.2.4`
3. build and verify artifacts from the release-ready code
4. merge `dev` into `main`
5. tag `main` with `v0.2.4`
6. publish GitHub Release assets
7. publish `ctx` to `crates.io`
8. publish `@alegau/ctx-bin@0.2.4` to npm
9. update and push the Homebrew tap formula
10. run public-channel smoke checks

`main` is the source of truth for the public release. GitHub Releases is the source of truth for installer downloads, checksum verification, and npm binary downloads.

## Public Channel Strategy

### GitHub Release

GitHub Release is the canonical public distribution surface for binary artifacts. The release must include:

- platform archives
- `SHA256SUMS`
- `release-manifest.json`

The artifact naming stays aligned with the current convention:

- `ctx-0.2.4-aarch64-apple-darwin.tar.gz`
- `ctx-0.2.4-x86_64-apple-darwin.tar.gz`
- `ctx-0.2.4-x86_64-unknown-linux-gnu.tar.gz`
- `ctx-0.2.4-x86_64-pc-windows-msvc.zip`

If a target is not actually shipped, the release notes and docs must say that explicitly instead of implying support.

### Cargo

The `ctx` crate should be published at `0.2.4` and remain installable with:

```bash
cargo install ctx
```

The release flow must verify that crate metadata is coherent before publication and that public documentation references the published path instead of source-only install wording.

### npm

The npm package should be published as:

```bash
npm i -g @alegau/ctx-bin
```

`@alegau/ctx-bin@0.2.4` must point to the `v0.2.4` GitHub Release artifacts. Its package version, README, install behavior, and update instructions must match the release.

### Homebrew

The Homebrew tap is:

- repository: `Alegau03/homebrew-ctx`
- user-facing tap: `Alegau03/ctx`

For a polished public path, the formula must be updated from the release-ready version and published to the tap repository so users can run:

```bash
brew tap Alegau03/ctx
brew install ctx
brew upgrade ctx
```

The tap update must use explicit version and checksum data so the formula is reproducible and auditable.

## Version Coherence

`0.2.4` must be the only public version number visible in:

- Rust workspace package version
- `crates/ctx-cli/Cargo.toml`
- `packages/ctx-bin/package.json`
- GitHub tag `v0.2.4`
- artifact filenames
- release notes
- Homebrew formula
- installation and update docs

No release should proceed if one channel still points at `0.1.0` while another points at `0.2.4`.

## Verification Gates

Before public publication, the release candidate must pass:

- Rust tests
- release artifact build
- archive verification against `SHA256SUMS`
- installed binary smoke
- OpenCode integration smoke
- `npm publish --dry-run`
- documentation consistency checks for install and update commands

After publication, the public surfaces must be verified with:

- GitHub Release asset presence
- crate publish success
- npm publish success
- Homebrew formula availability in the tap
- install/update smoke using the public commands where feasible

## Documentation Requirements

The following docs must agree on install and update behavior for `v0.2.4`:

- `README.md`
- `docs/install.md`
- `docs/commands.md`
- `docs/release-playbook.md`
- `guide.md`
- `packages/ctx-bin/README.md`

The docs should explicitly describe:

- `ctx update`
- `ctx update --check`
- channel-specific fallback commands
- GitHub Release as the binary source of truth
- the fact that GIF placeholders remain until post-release media capture

## Operational Constraints

- The existing `v0.1.0` GitHub Release remains intact and must not be overwritten.
- `v0.2.4` must be published under a new tag.
- Because README media is still pending, release logic and packaging should be completed first, then media capture can follow against the final shipped surface.
- Publication should happen only after `dev` is merged into `main`.

## Success Criteria

The release is considered complete when:

- `v0.2.4` exists on GitHub with verified assets
- `cargo install ctx` resolves to the new release
- `npm i -g @alegau/ctx-bin` installs the new release
- `brew tap Alegau03/ctx && brew install ctx` works from the public tap
- install and update docs describe the same commands everywhere
- the repo is ready for post-release README GIF insertion and announcement work
