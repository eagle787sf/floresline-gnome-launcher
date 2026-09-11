#!/usr/bin/env bash
# Install Floresline GNOME launcher + optional Super-key extension
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
EXT_UUID="floresline-super-launcher@floresline"
EXT_DIR="${HOME}/.local/share/gnome-shell/extensions/${EXT_UUID}"

mkdir -p "$BIN_DIR" "$APP_DIR"

install -m 755 "$ROOT/bin/floresline-launcher" "$BIN_DIR/floresline-launcher"
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

# Extension
mkdir -p "$EXT_DIR"
install -m 644 "$ROOT/extension/${EXT_UUID}/metadata.json" "$EXT_DIR/metadata.json"
install -m 644 "$ROOT/extension/${EXT_UUID}/extension.js" "$EXT_DIR/extension.js"

# Keybindings: Super+Space launcher (reliable); keep Super+/ too
SCHEMA=org.gnome.settings-daemon.plugins.media-keys
# Merge carefully — append customs if missing
python3 - << PY
import subprocess, ast, os
home = os.path.expanduser("~")
schema = "org.gnome.settings-daemon.plugins.media-keys"
raw = subprocess.check_output(["gsettings", "get", schema, "custom-keybindings"], text=True).strip()
try:
    bindings = list(ast.literal_eval(raw))
except Exception:
    bindings = []

def ensure(path, name, command, binding):
    global bindings
    if path not in bindings:
        bindings.append(path)
    base = f"{schema}.custom-keybinding:{path}"
    subprocess.check_call(["gsettings", "set", base, "name", name])
    subprocess.check_call(["gsettings", "set", base, "command", command])
    subprocess.check_call(["gsettings", "set", base, "binding", binding])

# Use high custom indices to reduce collisions
ensure(
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/floresline-launcher-space/",
    "Floresline Launcher",
    f"{home}/.local/bin/floresline-launcher-toggle",
    "<Super>space",
)
ensure(
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/floresline-launcher-slash/",
    "Floresline Launcher (slash)",
    f"{home}/.local/bin/floresline-launcher-toggle",
    "<Super>slash",
)
fmt = "[" + ", ".join(f"'{b}'" for b in bindings) + "]"
subprocess.check_call(["gsettings", "set", schema, "custom-keybindings", fmt])

# Move input-source off Super+Space if still there
try:
    src = subprocess.check_output(
        ["gsettings", "get", "org.gnome.desktop.wm.keybindings", "switch-input-source"],
        text=True,
    )
    if "<Super>space" in src:
        subprocess.check_call([
            "gsettings", "set",
            "org.gnome.desktop.wm.keybindings", "switch-input-source",
            "['XF86Keyboard']",
        ])
except Exception:
    pass

# Enable extension in gsettings
raw = subprocess.check_output(["gsettings", "get", "org.gnome.shell", "enabled-extensions"], text=True).strip()
try:
    exts = list(ast.literal_eval(raw))
except Exception:
    exts = []
uuid = "floresline-super-launcher@floresline"
if uuid not in exts:
    exts.append(uuid)
fmt = "[" + ", ".join(f"'{e}'" for e in exts) + "]"
subprocess.check_call(["gsettings", "set", "org.gnome.shell", "enabled-extensions", fmt])
print("Keybindings + extension enable list updated.")
PY

update-desktop-database "$APP_DIR" 2>/dev/null || true

cat << MSG

Installed:
  ~/.local/bin/floresline-launcher
  ~/.local/bin/floresline-launcher-toggle
  ~/.local/share/gnome-shell/extensions/${EXT_UUID}/

Shortcuts:
  Super + Space     → launcher
  Super + /         → launcher
  Super (alone)     → launcher AFTER you log out/in (extension)

GNOME Wayland requires a session restart to load new extensions:
  Log out → log back in
  Then press Super.

Test without Super:  floresline-launcher-toggle
MSG
