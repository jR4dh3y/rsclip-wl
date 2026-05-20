# rsclip

rsclip is a small Rust clipboard manager for Wayland desktops. It uses a low-memory daemon to
capture clipboard content and a separate resident GTK4 UI that starts on demand, stays warm,
and is activated by later `rsclip` invocations.

## Current scope

- SQLite-backed text and image history.
- `rsclipd store --mime ...` for manual or watcher-driven ingestion.
- `rsclipd watch` to spawn `wl-paste --watch` text and PNG watchers.
- Text, link, and color classification.
- Image payload storage under XDG data directories.
- Resident GTK4 history window with search, filters, preview, copy, and auto-paste.
- OCR command plumbing through `rsclipd ocr`.

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
printf 'hello from rsclip' | cargo run -p rsclip-daemon --bin rsclipd -- store --mime text/plain
cargo run -p rsclip-daemon --bin rsclipd -- list
```

Run the watcher:

```bash
cargo run -p rsclip-daemon --bin rsclipd -- watch
```

Open the UI:

```bash
cargo run -p rsclip-ui --bin rsclip
```

The first `rsclip` launch starts the UI process. Later invocations activate the existing
process instead of cold-starting another overlay:

```bash
rsclip              # show the resident UI
rsclip show         # show the resident UI
rsclip toggle       # hide if visible, show if hidden
rsclip quit-ui      # stop the resident UI process
rsclip list         # print history without starting GTK
```

Keep `rsclipd watch` as the headless service. The UI and daemon are separate processes; the
daemon stores history in SQLite and notifies the UI over the existing Unix datagram socket.

Install the service and desktop file by adapting the files under `packaging/`.

## Release and AUR

This repository can publish a binary AUR package from GitHub release assets.

- Build the release archive locally with `./scripts/build-release-archive.sh 0.1.3`.
- The AUR package definition lives under `packaging/aur/rsclip-bin`.
- Pushing a matching Git tag such as `v0.1.3` triggers GitHub Actions to publish the
  archive and update the `rsclip-bin` AUR package.
