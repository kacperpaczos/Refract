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
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

import { SnapFinderSearchProvider } from './snapFinderSearchProvider.js';

export default class SnapdSearchProviderExtension extends Extension {
    enable() {
        const { snapProtoHandler } = SnapFinderSearchProvider;
        if (!snapProtoHandler) {
            log('No snap protocol handler, skip Snap search provider');
            return;
        }

        this.snapFinder = new SnapFinderSearchProvider();
        this._registerProvider(this.snapFinder);
    }

    disable() {
        if (!this.snapFinder)
            return;

        this._unregisterProvider(this.snapFinder);
        delete this.snapFinder;
    }

    _registerProvider(provider) {
        this.searchResults._registerProvider(provider);
        provider.init();
    }

    _unregisterProvider(provider) {
        this.searchResults._unregisterProvider(provider);
        provider.destroy();
    }

    get searchResults() {
        return Main.overview._overview.controls._searchController._searchResults;
    }
}
