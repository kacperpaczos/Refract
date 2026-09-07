/* extension.js
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

import {
    GLib,
    Shell,
} from './dependencies/gi.js';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import {DBusServer} from './dbusServer.js';
import {PromptsHandler} from './promptsHandler.js';

export default class SnapdPromptingDialog extends Extension {
    enable() {
        this._dbusServer = new DBusServer();
        this._promptsHandler = new PromptsHandler(this._dbusServer);

        if (GLib.getenv('SNAPD_PROMPTING_DEBUG')) {
            Shell.util_spawn_async(
                null, ['python3', `${this.path}/../tools/permission-dialog-launcher.py`], null,
                GLib.SpawnFlags.SEARCH_PATH);
        }
    }

    disable() {
        this._dbusServer?.destroy();
        this._dbusServer = null;
        this._promptsHandler?.destroy();
        this._promptsHandler = null;
    }
}
