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

import St from 'gi://St'
import GLib from 'gi://GLib'
import Gio from 'gi://Gio'

import { gettext as _ } from 'resource:///org/gnome/shell/extensions/extension.js';

Gio._promisify(Gio.AppInfo, 'get_default_for_uri_scheme_async');
Gio._promisify(Gio.AppInfo.prototype, 'launch_uris_async');

const SEARCH_ENGINES = Object.freeze({
  GOOGLE: 'https://www.google.com/search?ie=utf-8&oe=utf-8&q={{searchTerms}}',
  GOOGLE_UBUNTU: 'https://www.google.com/search?client=ubuntu&channel=fs&ie=utf-8&oe=utf-8&q={{searchTerms}}',
  BING: 'https://www.bing.com/search?q={{searchTerms}}',
  BAIDU: 'https://www.baidu.com/s?ie=utf-8&wd={{searchTerms}}',
  DUCK_DUCK_GO: 'https://duckduckgo.com/?q={{searchTerms}}',
});

export class WebSearcherSearchProvider {

  constructor(browserAppInfo, cancellable) {
    this.appInfo = browserAppInfo;
    this._cancellable = cancellable;
  }

  destroy() {
    delete this._cancellable;
    delete this.appInfo;
  }

  static async getDefaultBrowser(cancellable) {
    return Gio.AppInfo.get_default_for_uri_scheme_async('https', cancellable);
  }

  get id() {
    return this.appInfo?.get_id() ?? 'ubuntu-web-search-search-provider';
  }

  get isRemoteProvider() {
    return false;
  }

  get canLaunchSearch() {
    return true;
  }

  async getInitialResultSet(terms, _cancellable) {
    return [terms.join(' ')];
  }

  async getResultMetas(results, _cancellable) {
    const [terms] = results;

    return [
      {
        id: terms,
        name: _('Search online'),
        description: _('Search "%s" on the web').format(terms),
        createIcon: (size) => {
          return new St.Icon({
            iconName: 'system-search-symbolic',
            iconSize: size,
            trackHover: true,
          });
        },
      }
    ];
  }

  getSubsearchResultSet(_previousResults, terms, cancellable) {
    return this.getInitialResultSet(terms, cancellable);
  }

  filterResults(results, max) {
    return results.slice(0, max);
  }

  activateResult(id, terms) {
    this._launchBrowser(terms).catch(logError);
  }

  launchSearch(terms) {
    this._launchBrowser(terms).catch(logError);
  }

  async _launchBrowser(terms) {
    const browserApp = await WebSearcherSearchProvider.getDefaultBrowser(this._cancellable);

    if (!browserApp) {
      log("Impossible to find a default browser in this system");
      return;
    }

    if (browserApp.get_id() !== this.appInfo.get_id())
      this.appInfo = browserApp;

    const searchTerms = terms.join(' ');
    const launchContext = global.create_app_launch_context(0, -1);

    const appId = this.appInfo.get_id();
    if (appId == 'firefox.desktop' ||
        appId == 'firefox_firefox.desktop' ||
        appId == 'org.mozilla.firefox.desktop') {
      let baseCommandLine = this.appInfo.get_commandline().replaceAll(/\s+%[A-z]/g, '');
      const cmdLine = [baseCommandLine, '--new-tab', '--search', GLib.shell_quote(searchTerms)]

      const appInfo = Gio.AppInfo.create_from_commandline(cmdLine.join(' '),
        this.appInfo.get_name(),
        Gio.AppInfoCreateFlags.SUPPORTS_STARTUP_NOTIFICATION)
      appInfo.launch([], launchContext);
    } else {
      // TODO: add settings
      const searchUri = SEARCH_ENGINES['GOOGLE_UBUNTU'].replaceAll(
        '{{searchTerms}}', encodeURIComponent(searchTerms));

      await this.appInfo.launch_uris_async([searchUri], launchContext, this._cancellable);
    }
  }
}
