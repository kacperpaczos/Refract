/*
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
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

import St from 'gi://St'
import Gio from 'gi://Gio'
import GLib from 'gi://GLib'
import GioUnix from 'gi://GioUnix'
import Snapd from 'gi://Snapd?version=2'
import {CancellableChild} from './cancellableChild.js';

const SNAP_STORE_DESKTOP_ID = 'snap-store_snap-store.desktop';

Gio._promisify(Snapd.Client.prototype, 'find_async');

export class SnapFinderSearchProvider /* extends SearchProvider */ {
    constructor() {
        const { snapProtoHandler } = SnapFinderSearchProvider;

        if (snapProtoHandler?.should_show())
            this.appInfo = snapProtoHandler;

        this._client = new Snapd.Client();
        this._searchResults = new Map();
    }

    init() {
        this._displayConnectId = this.display.connect('notify::visible', () =>
                this._searchResults.clear());
        this.display.connect('destroy', () => (this._displayConnectId = 0));
    }

    destroy() {
        if (this._displayConnectId)
            this.display?.disconnect(this._displayConnectId);

        this._lastCancellable?.cancel();
        this._lastCancellable = null;
        this._client = null;
    }

    static get snapProtoHandler() {
        const snapStore = GioUnix.DesktopAppInfo.new(SNAP_STORE_DESKTOP_ID);
        if (snapStore?.supports_uris())
            return snapStore

        return Gio.AppInfo.get_default_for_uri_scheme('snap');
    }

    get id() {
        return this.appInfo?.get_id() ?? SnapFinderSearchProvider?.get_id() ??
            'snapd-search-provider';
    }

    get isRemoteProvider() {
        return true;
    }

    get canLaunchSearch() {
        return this.appInfo?.get_id() === SNAP_STORE_DESKTOP_ID;
    }

    async XUbuntuCancel(_cancellable) {
        this._lastCancellable?.cancel();
        delete this._lastCancellable;
    }

    async getInitialResultSet(terms, cancellable) {
        const networkMonitor = Gio.NetworkMonitor.get_default();
        if (!networkMonitor.networkAvailable)
            return [];

        if (networkMonitor.get_network_metered())
            return [];

        try {
            this._lastCancellable?.cancel();
            cancellable = new CancellableChild(cancellable);
            this._lastCancellable = cancellable;

            await new Promise((resolve, reject) => {
                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
                    if (cancellable.is_cancelled()) {
                        reject(new GLib.Error(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED,
                            "Search cancelled"));
                        return GLib.SOURCE_REMOVE;
                    }

                    resolve();
                    return GLib.SOURCE_REMOVE;
                });
            });

            const [results,] = await this._client.find_async(Snapd.FindFlags.NONE,
                terms.join(' '), cancellable);

            return (results?.filter(s => s.status !== Snapd.SnapStatus.INSTALLED) ?? []).map(s => {
                this._searchResults.set(s.name, s);
                return s.name;
            });
        } catch (e) {
            if (!e.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED))
                logError(e, `Failed to search snaps with terms: ${terms.join(' ')}`);

            return [];
        }
    }

    async getSubsearchResultSet(_previousResults, terms, cancellable) {
        return this.getInitialResultSet(terms, cancellable);
    }

    async getResultMetas(snaps) {
        return snaps.map(snapName => {
            const snap = this._searchResults.get(snapName) ??
                { id: snapName, name: snapName };

            // Snaps provide an ID, but the name is also unique and can
            // be used with the snap URI handler, so we can use it as ID
            // to avoid having to deal with the snap ID in the activate handler.
            const { name: id, name, description, icon } = snap;

            return {
                id,
                name,
                description,
                createIcon: (size) => {
                    return new St.Icon({
                        gicon: (() => {
                            if (!icon)
                                return null;

                            if (icon.startsWith('/'))
                                return new Gio.FileIcon({ file: Gio.File.new_for_path(icon) });

                            if (icon.includes(':/'))
                                return new Gio.FileIcon({ file: Gio.File.new_for_uri(icon) });

                            return null;
                        })(),
                        fallbackIconName: 'application-vnd.snap-symbolic',
                        iconSize: size,
                        styleClass: 'show-apps-icon',
                        trackHover: true,
                    });
                },
            }
        });
    }

    filterResults(results, max) {
        return results.slice(0, max);
    }

    activateResult(snapName, terms) {
        this._openHandler(snapName).catch(logError);
    }

    async _openHandler(snapName) {
        let snapHandler = this.appInfo;

        if (!snapHandler) {
            const { snapProtoHandler } = SnapFinderSearchProvider;

            if (!snapProtoHandler) {
                logError(new Error("Impossible to find a default browser in this system"));
                return;
            }
        }

        try {
            const launchContext = global.create_app_launch_context(0, -1);
            await snapHandler.launch_uris_async([`snap://${snapName}`], launchContext, null);
        } catch (e) {
            logError(e, `Launching handler for ID: '${snapName}'`)
        }
    }

    launchSearch(terms) {
        if (this.appInfo?.get_id() !== SNAP_STORE_DESKTOP_ID) {
            // We may also fallback to https://snapcraft.io/store?q=${searchTerms}
            // but let's not do it for now.
            return;
        }

        const commandLine = [
            this.appInfo.get_commandline().replaceAll(/\s+%[A-z]/g, ''),
            '--search',
            GLib.shell_quote(terms.join(' ')),
        ]

        const launchContext = global.create_app_launch_context(0, -1);
        const appInfo = Gio.AppInfo.create_from_commandline(commandLine.join(' '),
            this.appInfo.get_name(),
            Gio.AppInfoCreateFlags.SUPPORTS_STARTUP_NOTIFICATION)
        appInfo.launch([], launchContext);
    }
};
