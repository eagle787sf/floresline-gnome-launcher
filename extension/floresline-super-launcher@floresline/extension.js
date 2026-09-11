/* Floresline Super Launcher — GNOME Shell 45+ (ESM)
 * Binds the Super (overlay) key to the Floresline launcher toggle.
 */
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class FloreslineSuperLauncher extends Extension {
    enable() {
        this._toggle = `${GLib.get_home_dir()}/.local/bin/floresline-launcher-toggle`;

        Main.wm.setCustomKeybindingHandler(
            'overlay-key',
            Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
            () => this._onSuper()
        );
    }

    _onSuper() {
        if (Main.overview.visible) {
            Main.overview.hide();
            return;
        }
        try {
            GLib.spawn_command_line_async(this._toggle);
        } catch (e) {
            console.error(`Floresline Super Launcher: ${e}`);
        }
    }

    disable() {
        if (Main.sessionMode.hasOverview) {
            Main.wm.setCustomKeybindingHandler(
                'overlay-key',
                Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
                Main.overview.toggle.bind(Main.overview)
            );
        }
    }
}
