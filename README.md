# Seythm

Terminal-native 6-key rhythm game with shell-first UI, bundled charts, osu!mania import, startup splash branding, artwork preview, and autoplay demo mode.

## Status

Alpha. The core loop is playable, but the product is still moving quickly. Expect rough edges.

## Run

Build and launch:

```bash
cargo run
```

Run the autoplay demo mode:

```bash
cargo run -- autoplay
```

Import an extracted osu!mania folder:

```bash
cargo run -- --import-osu /path/to/extracted/beatmap-folder
```

If you build a release binary, the executable name is:

```bash
./target/release/seythm
```

## Controls

- `Enter`: start song / confirm
- `↑/↓`: move selection
- `/`: open unified search
- `R`: cycle browse sort
- `I`: switch to imported charts
- `S`: open settings
- `T`: cycle theme
- `Esc`: back / close search / quit from browse

## Packaging For GitHub Releases

Each release archive should include:

- `seythm` executable
- `assets/`
- `README.md`
- `LICENSE`

Recommended release layout:

```text
seythm-linux-x86_64/
  seythm
  assets/
  README.md
  LICENSE
```

## Source Repository Contents

Keep these in the repo:

- `src/`
- `assets/`
- `tests/`
- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `LICENSE`

Do not publish build artifacts such as `target/`.

## Platform Notes

Current status:

- Linux: primary development target, most reliable
- macOS: should be workable, but needs real device testing
- Windows: code should build, but terminal image preview and device behavior need validation

The game still runs in terminals without graphics support, but artwork preview falls back to text-only mode. For the best experience, use a terminal with kitty-style image protocol support such as `kitty` or `ghostty`.

## Product Config

Branding is product-side, not user-editable in settings. The startup name, tagline, and ASCII logo come from:

`assets/brand.toml`

## Before Publishing

- Add a real `LICENSE` file. The repo does not have one yet.
- Build per-platform release archives instead of shipping `target/` directly.
