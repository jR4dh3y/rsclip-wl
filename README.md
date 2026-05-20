# Clipvault

Clipvault is a small Rust clipboard manager for Wayland desktops. It follows the design in
`/home/radhey/code/dots-niri/plan.md`: a low-memory daemon captures clipboard content and a
separate GTK4 UI opens on demand.

## Current scope

- SQLite-backed text and image history.
- `clipvaultd store --mime ...` for manual or watcher-driven ingestion.
- `clipvaultd watch` to spawn `wl-paste --watch` text and PNG watchers.
- Text, link, and color classification.
- Image payload storage under XDG data directories.
- Basic GTK4 history window with search, filters, preview, copy, and auto-paste.
- OCR command plumbing through `clipvaultd ocr`.

## Build

```bash
cargo build
```

On Arch/CachyOS, the GTK4 layer-shell system dependency is required for the overlay UI:

```bash
sudo pacman -S gtk4-layer-shell
```

Runtime tools expected by the full flow:

```bash
wl-copy wl-paste wtype tesseract
```

## Try it

Manual storage:

```bash
printf 'hello from clipvault' | cargo run -p clipvault-daemon --bin clipvaultd -- store --mime text/plain
cargo run -p clipvault-daemon --bin clipvaultd -- list
```

Run the watcher:

```bash
cargo run -p clipvault-daemon --bin clipvaultd -- watch
```

Open the UI:

```bash
cargo run -p clipvault-ui --bin clipvault
```

Install the service and desktop file by adapting the files under `packaging/`.

## Release and AUR

This repository can publish a binary AUR package from GitHub release assets.

- Build the release archive locally with `./scripts/build-release-archive.sh 0.1.0`.
- The AUR package definition lives under `packaging/aur/rsclip-wl-bin`.
- Pushing a matching Git tag such as `v0.1.0` triggers GitHub Actions to publish the
  archive and update the `rsclip-wl-bin` AUR package.
