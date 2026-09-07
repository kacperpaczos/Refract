/* dbusServer.js
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
 * Author: Marco Trevisan <marco@ubuntu.com>
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

import {
    Gio,
    GObject,
} from './dependencies/gi.js';


const SnapdPromptingInterface = 'com.canonical.Shell.PermissionPrompting';
const SnapdPromptingObjectPath = `/${SnapdPromptingInterface.replaceAll('.', '/')}`;

const SNAPD_PROMPTING_INTERFACE = `
<node>
    <interface name="${SnapdPromptingInterface}">
        <method name="Prompt">
            <arg name="snap_app_id" type="s" direction="in"/>
            <arg name="app_pid" type="t" direction="in"/>
        </method>
    </interface>
</node>
`;

export const DBusServer = GObject.registerClass({
    Signals: {
        'prompt-request': {
            param_types: [GObject.TYPE_STRING, GObject.TYPE_STRING, GObject.TYPE_UINT64],
        },
    },
}, class DBusServer extends GObject.Object {
    constructor() {
        super();

        // TODO: Make all this asyc
        this._dbusObject = Gio.DBusExportedObject.wrapJSObject(
            SNAPD_PROMPTING_INTERFACE, this);
        try {
            this._dbusObject.export(Gio.DBus.session, SnapdPromptingObjectPath);
        } catch (e) {
            logError(e, `Failed to export ${SnapdPromptingObjectPath}`);
        }

        this._ownName = Gio.DBus.session.own_name(SnapdPromptingInterface,
            Gio.BusNameOwnerFlags.NONE, null, () => this._lostName());
    }

    destroy() {
        if (this._ownName)
            Gio.DBus.session.unown_name(this._ownName);

        try {
            this._dbusObject.unexport();
        } catch (e) {
            logError(e, `Failed to unexport ${SnapdPromptingObjectPath}`);
        }

        this._dbusObject.run_dispose();
        delete this._dbusObject;
    }

    _lostName(name) {
        console.error(`Name "${name}" lost`);
        delete this._ownName;
    }

    PromptAsync([snapAppID, appPid], invocation) {
        this.emit('prompt-request', invocation.get_sender(), snapAppID, appPid);

        try {
            invocation.return_value(null);
        } catch (e) {
            invocation.return_gerror(e);
        }
    }
});
