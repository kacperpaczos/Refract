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
import Gio from 'gi://Gio'

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import { WebSearcherSearchProvider } from './webSearcherSearchProvider.js';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
export * as Util from 'resource:///org/gnome/shell/misc/util.js';

export default class SnapdSearchProviderExtension extends Extension {
  async enable() {
    const cancellable = new Gio.Cancellable();
    this._cancellable = cancellable;

    try {
      const defaultBrowser = await WebSearcherSearchProvider.getDefaultBrowser(cancellable);

      // Ignore if default browser is Web as it already provides a search provider...
      if (!defaultBrowser || defaultBrowser.get_id() === 'org.gnome.Epiphany.desktop') {
        log('No default browser found or default browser is Web, skipping web search provider');
        delete this._cancellable;
        return;
      }

      this.webSearcher = new WebSearcherSearchProvider(defaultBrowser, cancellable);
      this.searchResults._registerProvider(this.webSearcher);
    } catch (error) {
      if (error.matches(Gio.IOErrorEnum.CANCELLED))
        return;

      logError(error, 'Error while enabling web search provider');
    }
  }

  disable() {
    this._cancellable?.cancel();
    delete this._cancellable;

    if (this.webSearcher) {
      this.searchResults._unregisterProvider(this.webSearcher);
      this.webSearcher?.destroy();
      delete this.webSearcher;
    }
  }

  get searchResults() {
    return Main.overview._overview.controls._searchController._searchResults;
  }
}
