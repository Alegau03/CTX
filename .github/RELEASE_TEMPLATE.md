## Highlights

- 
- 
- 

## Install

```bash
shasum -a 256 -c SHA256SUMS
tar -xzf ctx-<version>-<target>.tar.gz
sudo install -m 0755 ctx-<version>-<target>/ctx /usr/local/bin/ctx
ctx doctor
```

For Windows, download the matching `.zip` release and add `ctx.exe` to your PATH.
If this release only ships a macOS Apple Silicon archive, direct other platforms to source install.

## OpenCode Quick Start

```bash
ctx init
ctx index
ctx opencode install
opencode
```

Inside OpenCode:

```text
/ctx-memory-bootstrap
/ctx-memory-search auth
/ctx-pack fix refresh token bug
```

## Benchmark Evidence

- `demo/fixtures/opencode-auth-lab/benchmarks/report.md`
- `demo/fixtures/opencode-auth-lab/benchmarks/report.json`

## Verification

```bash
scripts/release/verify-artifact.sh dist/ctx-<version>-<target>.tar.gz dist/SHA256SUMS
```

## Notes

- OpenCode is the primary daily path.
- Graph memory is the preferred replacement for giant instruction markdown rereads.
