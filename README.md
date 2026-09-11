# Floresline GNOME Launcher

Pop!_OS-style **fuzzy app launcher** for modern GNOME (45–50), plus an optional Shell extension that opens it with the **Super** key.

Inspired by [Pop Launcher Super-Key](https://extensions.gnome.org/extension/4797/pop-launcher-super-key/) — that extension targets older GNOME and Pop’s launcher. This project is a maintained alternative for **Ubuntu / stock GNOME Wayland**.

![GNOME](https://img.shields.io/badge/GNOME-45%20%7C%2046%20%7C%2047%20%7C%2048%20%7C%2049%20%7C%2050-4A86CF?logo=gnome&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- **Fuzzy search** across `.desktop` apps (system, Flatpak, Snap, `~/.local`)
- **Pop-style prefixes** for web search and optional extras
- **Web search**: `? query`, `ddg …`, `google …`, `gs …` (opens in the browser); bare `https://…` URLs open directly
- **Recents** — frequently/recently launched apps rise to the top (empty query prefers them)
- **Alt+1…9** launch the nth visible result (number badges on the first 9 rows)
- **Search field keeps focus** while you type multi-letter queries
- **Keyboard nav**: `↑` `↓` select · `Enter` launch · `Esc` close
- **Pop-like shortcuts**: `Super+Space`, `Super+/`
- **Optional Super-key extension** (GJS): Super alone opens the launcher (`Super+A` still opens the app grid). **Log out and back in** once after install (Wayland)
- **No root** required for install
- GTK4 · Wayland-friendly
- Optional extras: `~/.config/floresline-launcher/extras.toml`

## Requirements

| Component | Requirement |
|-----------|-------------|
| Desktop | GNOME Shell **45–50** (tested on **50.1** / Ubuntu) |
| Session | Wayland or X11 |
| Runtime | Rust + GTK4 binary (preferred when built) or Python 3.10+ / PyGObject |
| Super-key bind | Log out/in once after installing the GJS extension (Wayland) |

## Quick install

```bash
git clone https://github.com/eagle787sf/floresline-gnome-launcher.git
cd floresline-gnome-launcher
./install.sh
```

`install.sh` installs the **Rust release binary** when it has been built (`rust/floresline-launcher/target/release/floresline-launcher`) and keeps Python as `floresline-launcher-python`. See [docs/RUST.md](docs/RUST.md) for deps and `cargo build --release`.

Then **log out and back in** (needed for the Super-key extension on Wayland).

### Try without Super

```bash
floresline-launcher-toggle
# or
~/.local/bin/floresline-launcher
```

## Shortcuts

| Shortcut | Action |
|----------|--------|
| `Super` | Launcher *(after logout/in with the GJS extension)* |
| `Super` + `Space` | Launcher |
| `Super` + `/` | Launcher |
| `Super` + `A` | GNOME app grid |
| `↑` `↓` | Move selection |
| `Enter` | Launch / open search |
| `Alt` + `1`…`9` | Launch nth result |
| `?` / `ddg` / `gs` / `google` | Web search |
| `https://…` | Open URL |
| `Esc` | Close |

> **Keychron / Mac-layout boards:** set the hardware switch to **Windows** on Linux so Super is the Win key. See [docs/KEYCHRON.md](docs/KEYCHRON.md).

## What gets installed

```
~/.local/bin/floresline-launcher
~/.local/bin/floresline-launcher-toggle
~/.local/share/applications/floresline-launcher.desktop
~/.local/share/gnome-shell/extensions/floresline-super-launcher@floresline/
```

## Uninstall

```bash
./uninstall.sh
```

Log out/in to unload the extension.

## Project layout

```
bin/                      # Python launcher fallback + toggle scripts
extension/                # GNOME Shell Super-key extension (ESM / GJS, 45+)
rust/floresline-launcher/ # Rust + GTK4 launcher (v0.2.6)
packaging/                # Prebuilt .shell-extension.zip
install.sh / uninstall.sh
docs/                     # Extra notes (incl. RUST.md)
```

## Rust port

A native **Rust + GTK4** rewrite lives under `rust/floresline-launcher/` (v0.2.6). `install.sh` prefers the release binary when present and keeps Python as `floresline-launcher-python`. See [docs/RUST.md](docs/RUST.md) for deps and `cargo build --release`.

## Manual extension zip

```bash
gnome-extensions install -f packaging/floresline-super-launcher@floresline.shell-extension.zip
# then enable in Extensions app, log out/in
```

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Super still opens Activities | Log out/in; confirm extension enabled in *Extensions* |
| Can’t type more than one letter | Update to latest launcher (focus fix) |
| `Super+Space` switches keyboard layout | Install script remaps input-source off Super+Space |
| Extension “doesn’t exist” in CLI until reboot | Normal on Wayland until new session |
| No apps listed | Ensure `.desktop` files exist under `/usr/share/applications` or Flatpak exports |

## Relation to Pop Launcher Super-Key

[extensions.gnome.org #4797](https://extensions.gnome.org/extension/4797/pop-launcher-super-key/) binds **Pop Launcher** to Super and is largely unmaintained for GNOME 45+. This repo provides:

1. A self-contained GTK4 launcher (no Pop packages)
2. A small Super-key GJS extension for GNOME **45–50**

## License

MIT — see [LICENSE](LICENSE).

## Credits

- Pop!_OS / System76 for the launcher UX inspiration  
- GNOME Shell extension ESM pattern (45+)
