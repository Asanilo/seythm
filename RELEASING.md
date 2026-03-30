# Releasing Seythm

This document describes the public release workflow for Seythm.

## Current Support Matrix

| Platform | Build Status | Release Status | Notes |
| --- | --- | --- | --- |
| Linux x86_64 | verified | published | primary tested target |
| macOS Apple Silicon / Intel | expected | not yet published | needs terminal and audio validation |
| Windows x86_64 | expected | not yet published | needs terminal image preview validation |

## Release Contents

Each release archive should contain:

- `seythm` or platform-specific executable
- `assets/`
- `README.md`
- `LICENSE`

Recommended archive layouts:

```text
seythm-linux-x86_64/
  seythm
  assets/
  README.md
  LICENSE
```

```text
seythm-macos-universal/
  seythm
  assets/
  README.md
  LICENSE
```

```text
seythm-windows-x86_64/
  seythm.exe
  assets/
  README.md
  LICENSE
```

## Build Commands

Linux:

```bash
cargo build --release
```

macOS:

```bash
cargo build --release
```

Windows with Rust GNU target installed:

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

Windows with MSVC target:

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

## Validation Checklist

Before publishing a release:

1. Run `cargo test`
2. Run `cargo build --release`
3. Launch the built binary and verify:
   - startup splash renders
   - song select opens
   - search works
   - a bundled song can start and reach results
   - `seythm autoplay` works
4. Verify assets are bundled with the archive
5. Verify README and LICENSE are included

## Platform Notes

Linux:

- Recommended for early public releases.
- Best artwork-preview experience is in `kitty` or `ghostty`.

macOS:

- Expected to work with the same release structure.
- Needs validation for audio device startup and terminal image fallback behavior.

Windows:

- Core app should build with the correct Rust target and toolchain.
- Terminal image preview should be treated as optional until validated.
- Text-only fallback should remain acceptable.

## Next Cross-Platform Improvements

- Add GitHub Actions to build Linux, macOS, and Windows artifacts automatically.
- Publish per-platform archives on every tagged release.
- Add release smoke tests for `autoplay`, search, and startup flow.
