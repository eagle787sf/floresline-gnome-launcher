#!/usr/bin/env bash
# Install Floresline GNOME launcher + optional Super-key extension
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
CFG_DIR="${HOME}/.config/floresline-launcher"
EXT_UUID="floresline-super-launcher@floresline"
EXT_DIR="${HOME}/.local/share/gnome-shell/extensions/${EXT_UUID}"

mkdir -p "$BIN_DIR" "$APP_DIR" "$CFG_DIR"

# Seed config.toml once (never overwrite user edits)
if [[ ! -f "$CFG_DIR/config.toml" ]]; then
  if [[ -f "$ROOT/config/config.example.toml" ]]; then
    install -m 644 "$ROOT/config/config.example.toml" "$CFG_DIR/config.toml"
    echo "Created $CFG_DIR/config.toml (edit this to change shortcuts / options)"
  fi
fi

RUST_BIN="$ROOT/rust/floresline-launcher/target/release/floresline-launcher"
if [[ -x "$RUST_BIN" ]]; then
  install -m 755 "$RUST_BIN" "$BIN_DIR/floresline-launcher"
  install -m 755 "$ROOT/bin/floresline-launcher" "$BIN_DIR/floresline-launcher-python"
  echo "Installed Rust launcher (Python fallback: floresline-launcher-python)"
else
  install -m 755 "$ROOT/bin/floresline-launcher" "$BIN_DIR/floresline-launcher"
  echo "Installed Python launcher (build rust/floresline-launcher for native binary)"
fi
install -m 755 "$ROOT/bin/floresline-launcher-toggle" "$BIN_DIR/floresline-launcher-toggle"

cat > "$APP_DIR/floresline-launcher.desktop" << DESK
[Desktop Entry]
Name=Floresline Launcher
Comment=Pop-style fuzzy app launcher
Exec=${BIN_DIR}/floresline-launcher-toggle
Icon=system-search
Terminal=false
Type=Application
Categories=Utility;
Keywords=launcher;pop;search;gnome;
DESK

# Extension files always installed; enabling is controlled by config.toml
mkdir -p "$EXT_DIR"
install -m 644 "$ROOT/extension/${EXT_UUID}/metadata.json" "$EXT_DIR/metadata.json"
install -m 644 "$ROOT/extension/${EXT_UUID}/extension.js" "$EXT_DIR/extension.js"

python3 - <<'PY'
import ast, os, re, subprocess
from pathlib import Path

home = Path.home()
cfg_path = home / ".config/floresline-launcher/config.toml"
text = cfg_path.read_text() if cfg_path.is_file() else ""

def get_bool(key, default):
    m = re.search(rf"(?m)^\s*{re.escape(key)}\s*=\s*(true|false)\s*$", text)
    if not m:
        return default
    return m.group(1) == "true"

def get_str(key, default):
    m = re.search(rf'(?m)^\s*{re.escape(key)}\s*=\s*"([^"]*)"\s*$', text)
    return m.group(1).strip() if m else default

preferred = get_str("preferred_binding", "Super+backslash")
bind_slash = get_bool("bind_super_slash", True)
bind_search = get_bool("bind_xf86_search", True)
super_alone = get_bool("enable_super_alone_extension", False)

# Human "Super+backslash" → gsettings "<Super>backslash"
def to_gsettings(binding: str) -> str:
    b = binding.strip()
    if b.startswith("<") and b.endswith(">"):
        # already looks like a chord fragment; accept full form if present
        pass
    b = b.replace(" ", "")
    # Normalize common spellings
    repl = {
        "Super+backslash": "<Super>backslash",
        "Super+\\": "<Super>backslash",
        "Super+slash": "<Super>slash",
        "Super+/": "<Super>slash",
        "Super+space": "<Super>space",
        "Super+Space": "<Super>space",
    }
    if b in repl:
        return repl[b]
    # Generic Super+Key
    m = re.match(r"(?i)super\+(.+)$", b)
    if m:
        key = m.group(1)
        key = {"\\": "backslash", "/": "slash"}.get(key, key)
        return f"<Super>{key}"
    if b.startswith("<Super>"):
        return b
    return "<Super>backslash"

primary = to_gsettings(preferred)
toggle = str(home / ".local/bin/floresline-launcher-toggle")
schema = "org.gnome.settings-daemon.plugins.media-keys"
raw = subprocess.check_output(["gsettings", "get", schema, "custom-keybindings"], text=True).strip()
try:
    bindings = list(ast.literal_eval(raw))
except Exception:
    bindings = []

def ensure(path, name, command, binding):
    if path not in bindings:
        bindings.append(path)
    base = f"{schema}.custom-keybinding:{path}"
    subprocess.check_call(["gsettings", "set", base, "name", name])
    subprocess.check_call(["gsettings", "set", base, "command", command])
    subprocess.check_call(["gsettings", "set", base, "binding", binding])

def clear_binding(path):
    """Leave path registered but unbind if we previously owned it."""
    if path not in bindings:
        return
    base = f"{schema}.custom-keybinding:{path}"
    try:
        subprocess.check_call(["gsettings", "set", base, "binding", "['']"])
    except Exception:
        try:
            subprocess.check_call(["gsettings", "set", base, "binding", ""])
        except Exception:
            pass

# Stable floresline paths (and legacy custom2/custom3 if present — we use named paths)
ensure(
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/floresline-launcher/",
    "Floresline Launcher",
    toggle,
    primary,
)

slash_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/floresline-launcher-slash/"
if bind_slash:
    ensure(slash_path, "Floresline Launcher (slash)", toggle, "<Super>slash")
else:
    # If previously set, clear
    if slash_path in bindings:
        base = f"{schema}.custom-keybinding:{slash_path}"
        subprocess.check_call(["gsettings", "set", base, "binding", ""])

search_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/floresline-launcher-search/"
if bind_search:
    ensure(search_path, "Floresline Launcher (Search key)", toggle, "XF86Search")

# Drop old Super+Space default path if it still points at us (avoid surprising dual binds)
legacy_space = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/floresline-launcher-space/"
if legacy_space in bindings and primary != "<Super>space":
    base = f"{schema}.custom-keybinding:{legacy_space}"
    try:
        cmd = subprocess.check_output(["gsettings", "get", base, "command"], text=True)
        if "floresline-launcher-toggle" in cmd:
            subprocess.check_call(["gsettings", "set", base, "binding", ""])
    except Exception:
        pass

fmt = "[" + ", ".join(f"'{b}'" for b in bindings) + "]"
subprocess.check_call(["gsettings", "set", schema, "custom-keybindings", fmt])

# Keep Activities on bare Super
try:
    subprocess.check_call(["gsettings", "set", "org.gnome.mutter", "overlay-key", "Super"])
except Exception:
    pass

# Extension enable/disable from config
uuid = "floresline-super-launcher@floresline"
raw = subprocess.check_output(["gsettings", "get", "org.gnome.shell", "enabled-extensions"], text=True).strip()
try:
    exts = list(ast.literal_eval(raw))
except Exception:
    exts = []
if super_alone:
    if uuid not in exts:
        exts.append(uuid)
    print("Super-alone extension: ENABLED (log out/in once on Wayland)")
else:
    exts = [e for e in exts if e != uuid]
    print("Super-alone extension: disabled (bare Super = GNOME Activities)")
fmt = "[" + ", ".join(f"'{e}'" for e in exts) + "]"
subprocess.check_call(["gsettings", "set", "org.gnome.shell", "enabled-extensions", fmt])

print(f"Primary binding: {preferred} → {primary}")
print(f"Super+/ backup: {bind_slash}")
print(f"XF86Search: {bind_search}")
print(f"Config: {cfg_path}")
PY

update-desktop-database "$APP_DIR" 2>/dev/null || true

# Human-readable summary from config
python3 - <<'PY'
from pathlib import Path
import re
p = Path.home()/".config/floresline-launcher/config.toml"
t = p.read_text() if p.is_file() else ""
def gs(k,d):
    m=re.search(rf'(?m)^\s*{k}\s*=\s*"([^"]*)"\s*$',t); return m.group(1) if m else d
def gb(k,d):
    m=re.search(rf'(?m)^\s*{k}\s*=\s*(true|false)\s*$',t); return (m.group(1)=='true') if m else d
bind=gs('preferred_binding','Super+backslash')
slash=gb('bind_super_slash',True)
alone=gb('enable_super_alone_extension',False)
print(f"""
Installed:
  ~/.local/bin/floresline-launcher
  ~/.local/bin/floresline-launcher-toggle
  ~/.config/floresline-launcher/config.toml   ← edit this, then re-run ./install.sh

Shortcuts (from your config):
  {bind}     → launcher (primary)
  Super+/    → launcher {'(on)' if slash else '(off)'}
  Super alone → {'launcher (extension; log out/in)' if alone else 'GNOME Activities only'}

Test:  floresline-launcher-toggle
""")
PY
