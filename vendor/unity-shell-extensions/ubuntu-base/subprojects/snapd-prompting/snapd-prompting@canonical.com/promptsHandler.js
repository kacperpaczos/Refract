/* promptsHandler.js
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
    GLib,
    Shell,
    GObject,
} from './dependencies/gi.js';

import * as Utils from './utils.js';
import {WindowsGroup} from './windowsGroup.js';

import {InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';
import {CloseDialog} from './dependencies/shell/ui.js';

const DEFAULT_PROMPTING_DBUS_NAME = 'com.canonical.Snapd.AppArmor.PromptingClient.Dialog';

export const PromptsHandler = GObject.registerClass(
class PromptsHandler extends Utils.DestroyableGObject {
    constructor(dbusServer) {
        super();

        this._requestIDs = 0;
        this._pendingRequests = [];
        this._windowsGroups = [];
        this._injectionManager = new InjectionManager();

        // FIXME: Drop the injection as soon we don't have more apps to track.
        const self = this;
        this._injectionManager.overrideMethod(CloseDialog.CloseDialog.prototype,
            'vfunc_show', originalMethod => {
                return function (...args) {
                    // eslint-disable-next-line no-invalid-this
                    if (self._windowsGroups.find(wg => wg.snapWindows.includes(this._window)))
                        return;

                    // eslint-disable-next-line no-invalid-this
                    originalMethod.apply(this, ...args);
                };
            });

        this._injectionManager.overrideMethod(Shell.AppSystem.prototype, 'get_running', originalMethod => {
            return function (...args) {
                const promptApps = self._windowsGroups.map(wg => wg.promptApp);
                // eslint-disable-next-line no-invalid-this
                const runningApps = originalMethod.call(this, ...args);
                return runningApps.filter(a => !promptApps.includes(a));
            };
        });

        this._windowsTracker = Shell.WindowTracker.get_default();

        Gio.DBus.watch_name(Gio.BusType.SESSION,
            DEFAULT_PROMPTING_DBUS_NAME,
            Gio.BusNameWatcherFlags.NONE,
            (_, _c, owner) => (this._promptDialogName = owner),
            () => (this._promptDialogName = null));

        dbusServer.connectObject('prompt-request', (_, ...args) =>
            this._onPromptRequest(...args), this);

        global.display.connectObject('window-created',
            () => this._processRequests(), this);
    }

    _onPromptRequest(sender, snapAppID, snapAppPid) {
        if (!this._checkSenderValidity(sender)) {
            console.log(`Ignoring prompt request from ${sender}`);
            return;
        }

        if (snapAppID.indexOf('_') < 0) {
            // If no full security profile has been provided, we assume it's
            // the main application, for now.
            snapAppID = `${snapAppID}_${snapAppID}`;
        }

        const id = this._requestIDs++;
        const request = {id, sender, snapAppID, snapAppPid};
        this._pendingRequests.push(request);

        request.timeoutID = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, () => {
            const idx = this._pendingRequests.findIndex(r => r.id === id);
            if (idx < 0)
                return GLib.SOURCE_REMOVE;

            this._pendingRequests.splice(idx, 1);
            return GLib.SOURCE_REMOVE;
        });

        this._processRequests();
    }

    _processRequests() {
        if (!this._pendingRequests?.length)
            return;

        const currentRequests = this._pendingRequests;
        this._pendingRequests = [];

        currentRequests.forEach(async req => {
            try {
                const {sender, snapAppID, snapAppPid, timeoutID} = req;
                await this._processPromptRequest(sender, snapAppID, snapAppPid);
                GLib.source_remove(timeoutID);
            } catch (e) {
                // Retry until timeout occurs.
                logError(e, `Prompt request cannot be handled`);
                this._pendingRequests.push(req);
            }
        });
    }

    async _processPromptRequest(sender, snapAppID, snapAppPid) {
        const promptWindow = await this._findPromptWindow(sender);

        if (!promptWindow)
            throw new Error(`Prompt Window for sender ${sender} not found`);

        const snapApp = this._windowsTracker.get_app_from_pid(snapAppPid);
        if (!snapApp)
            throw new Error(`Snap application for PID ${snapAppPid} not found`);

        const snapWindows = snapApp.get_windows().filter(
            w => w.get_pid() === snapAppPid && w.get_sandboxed_app_id() === snapAppID);

        if (!snapWindows?.length)
            throw new Error(`Snap window(s) with App ID ${snapAppID} and PID ${snapAppPid} not found`);

        const windowGroup = new WindowsGroup(promptWindow, snapApp, snapWindows);
        this._windowsGroups.push(windowGroup);
        promptWindow.connectObject('unmanaging', () => {
            this._windowsGroups = this._windowsGroups.filter(wg => wg !== windowGroup);
            windowGroup.destroy();
        }, this);
    }

    async _findPromptWindow(sender) {
        // FIXME: we need to have only one window in this way...
        // const promptWindow = Utils.getWindowByBusName(sender);
        // So, expose also the GTK Window object path or window ID in the API.
        const allWindows = global.display.list_all_windows();
        const promptWindow = allWindows.find(w =>
            w.get_gtk_unique_bus_name() === sender);

        if (promptWindow)
            return promptWindow;

        const senderPID = await Utils.getSenderPid(sender);
        return allWindows.find(w => w.get_pid() === senderPID);
    }

    _checkSenderValidity(sender) {
        // TODO:!
        // We need to ensure also that this sender is matching a known unique name...
        // So as first, set a name watcher, and only accept connections from that name.
        console.log('_checkSenderValidity: Expecting', this._promptDialogName, 'got', sender);
        // Maybe double-check that the PID of the prompting window matches the snap ID
        return true;
    }

    destroy() {
        this._windowsTracker = null;
        this._injectionManager.clear();
        this._injectionManager = null;
        this._pendingRequests.forEach(req => GLib.source_remove(req.timeoutID));

        super.destroy();
    }
});
