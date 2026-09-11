# Rust + GTK4 port

Native Rust rewrite of the Floresline launcher using [GTK4](https://www.gtk.org/) via [gtk-rs](https://gtk-rs.org/gtk4-rs/stable/latest/book/).

**Status:** v0.2.3 — desktop scan, fuzzy ranking + recents boost, `=` calculator, web prefixes (`?`/`ddg`/`gs`/`google` + bare URLs), Alt+1–9 badges, focus UX, CSS theme, pidfile. `install.sh` prefers `rust/floresline-launcher/target/release/floresline-launcher` when present and keeps the Python script as `floresline-launcher-python`.

The GNOME Shell Super-key extension stays **GJS** (`extension/`); it is not part of this crate. Repo `bin/floresline-launcher` remains the Python fallback.

## Ubuntu / Debian build dependencies

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libgtk-4-dev
```

Also need a recent Rust toolchain (`rustup` recommended):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# ensure ~/.cargo/bin is on PATH
```

## Build & install

```bash
cd rust/floresline-launcher
cargo build --release
# then from repo root:
./install.sh
# or manually:
install -m 755 target/release/floresline-launcher ~/.local/bin/floresline-launcher
```

```bash
cargo run --release   # launch once without installing
cargo check           # compile check only
```

If `cargo check` fails with missing `gtk4` / `gtk-4.0` pkg-config errors, install `libgtk-4-dev` (and `pkg-config`) as above.

## Crate layout

```
rust/floresline-launcher/
  Cargo.toml          # v0.2.3; gtk4 → `gtk` v4_12; shlex, meval, serde_json, urlencoding, toml
  src/
    main.rs           # GtkApplication UI: prefixes, Alt+1-9, hint bar
    calc.rs           # `=` calculator (gnome-calculator -s, meval/bc fallback)
    desktop.rs        # parse/load .desktop dirs (same filters as Python)
    fuzzy.rs          # score / rank + recents bonus
    launch.rs         # terminal / gtk-launch / shlex Exec / xdg-open / clipboard helpers
    recents.rs        # $XDG_STATE_HOME/floresline-launcher/recents.json
    extras.rs         # optional ~/.config/floresline-launcher/extras.toml
```

## Calculator (GNOME-native)

Type `= 2+2` (space optional; fullwidth `＝` also works). The result row shows the
**expression** as the title and the **value** as a dim subtitle. Enter copies the
result via the GTK clipboard (and `wl-copy`/`xclip` when present) and runs
`notify-send "Copied to clipboard"`.

Evaluation order:

1. `gnome-calculator -s` / `--solve` when installed (GNOME’s calc CLI)
2. Built-in `meval` (offline arithmetic)
3. `bc -l` if available

```bash
# Recommended for Pop/cosmic-like unit-aware calc parity on GNOME:
sudo apt install -y gnome-calculator
# optional Wayland clipboard helper (GTK clipboard is primary):
# sudo apt install -y wl-clipboard
```

Bump the `gtk` crate version and/or feature flag (e.g. `v4_14`) in `Cargo.toml` when you need newer GTK APIs. See the gtk-rs book for feature ↔ GTK version mapping.

## Docs

- [GTK](https://www.gtk.org/)
- [GTK4 getting started](https://docs.gtk.org/gtk4/getting_started.html)
- [gtk4-rs book](https://gtk-rs.org/gtk4-rs/stable/latest/book/)
