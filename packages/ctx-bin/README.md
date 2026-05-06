# @alegau/ctx-bin

Prebuilt CTX binary distribution for npm users.

## Install

```bash
npm i -g @alegau/ctx-bin
ctx help
ctx doctor
```

## Update

```bash
ctx update
npm update -g @alegau/ctx-bin
```

`ctx update` detects npm-based installs when possible and prints the package-manager command that matches this channel. The canonical manual npm upgrade path remains:

```bash
npm update -g @alegau/ctx-bin
```

The package downloads the matching GitHub Release artifact during `postinstall` instead of compiling Rust on the target machine.

## Environment Overrides

- `CTX_VERSION`: install a specific release version
- `CTX_REPO_SLUG`: override the GitHub repository slug
- `CTX_RELEASE_BASE_URL`: override the release download base URL
