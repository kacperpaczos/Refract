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
    Meta,
    Shell,
    GObject,
} from './dependencies/gi.js';

import {
    Main,
} from './dependencies/shell/ui.js';

import {InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';

import * as Utils from './utils.js';
import {SignalTracker} from './dependencies/shell/misc.js';

export const WindowsGroup = GObject.registerClass(
class WindowsGroup extends Utils.DestroyableGObject {
    constructor(promptWindow, snapApp, snapWindows) {
        super();

        this._injectionManager = new InjectionManager();

        this._snapApp = snapApp;
        this._snapWindows = snapWindows;

        this._promptWindow = promptWindow;
        this._promptApp = Shell.WindowTracker.get_default().get_window_app(promptWindow);

        this._lastFocusedSnapWindowSignals = new SignalTracker.TransientSignalHolder(this);

        this._injectionManager.overrideMethod(this._promptApp, 'launch', () => () => {});
        this._injectionManager.overrideMethod(this._promptApp, 'launch_action', () => () => {});
        this._injectionManager.overrideMethod(this._promptApp, 'open_new_window', () => () => {});
        this._injectionManager.overrideMethod(this._promptApp, 'create_icon_texture', () => {
            return function (...args) {
                return snapApp.create_icon_texture(...args);
            };
        });
        this._injectionManager.overrideMethod(this._promptApp, 'can_open_new-window', () => () => false);
        this._injectionManager.overrideMethod(this._promptApp, 'get_windows', () => () => []);

        this._injectionManager.overrideMethod(this._snapApp, 'get_windows', originalMethod => {
            return function (...args) {
                // eslint-disable-next-line no-invalid-this
                return [...originalMethod.call(this, ...args) ?? [], promptWindow];
            };
        });

        this._injectionManager.overrideMethod(this._snapApp, 'activate', () => () =>
            Main.activateWindow(this._promptWindow));
        this._injectionManager.overrideMethod(this._snapApp, 'activate_window', () => () =>
            Main.activateWindow(this._promptWindow));
        this._injectionManager.overrideMethod(this._snapApp, 'activate_full', () => () =>
            Main.activateWindow(this._promptWindow));

        this._snapWindows.forEach(w => {
            this._injectionManager.overrideMethod(w, 'has_attached_dialogs', () => () => true);
            Main.wm._checkDimming(w);

            const replicatedMethods = [
                'activate',
                'activate_with_focus',
                'activate_with_workspace',
                'focus',
                'make_above',
                'raise',
                'raise_and_make_recent_on_workspace',
            ];

            replicatedMethods.forEach(method => {
                this._injectionManager.overrideMethod(w, method, originalMethod => (...args) => {
                    originalMethod.call(w, ...args);
                    this._promptWindow[method](...args);
                });
            });

            const updatePromptWindowPosition = () => {
                this._lastRect = null;
                Main.activateWindow(this._promptWindow);
                this._adjustPromptPosition();
                Main.wm._checkDimming(w);
            };

            w.connectObject('focus', updatePromptWindowPosition,
                GObject.ConnectFlags.AFTER, this);

            w.connectObject('raised', updatePromptWindowPosition,
                GObject.ConnectFlags.AFTER, this);
        });

        this._promptWindow.connectObject('focus', () => () => {
            this._lastFocusedSnapWindow().focus(global.get_current_time());
        }, GObject.ConnectFlags.AFTER, this._lastFocusedSnapWindowSignals);
        this._promptWindow.connectObject('raised', () => () =>
            this._lastFocusedSnapWindow().raise(),
        GObject.ConnectFlags.AFTER, this._lastFocusedSnapWindowSignals);

        this._promptWindowInjections = new InjectionManager();
        this._promptWindowInjections.overrideMethod(this._promptWindow,
            'is_attached_dialog', () => () => true);
        this._promptWindowInjections.overrideMethod(this._promptWindow,
            'get_transient_for', () => () => this._lastFocusedSnapWindow());
        this._promptWindowInjections.overrideMethod(this._promptWindow,
            'find_root_ancestor', () => () => this._lastFocusedSnapWindow());

        this._promptWindow.set_type(Meta.WindowType.MODAL_DIALOG);
        this._promptWindow.hide_from_window_list();

        this._snapApp.emit('windows-changed');

        // track the prompt window for closed signal...
        // And destroy everything...

        // TODO: Track all the windows, although they should not change.
        // Even though a snap could still open another window with different PID.
        // But we show the dialog over all of them...

        // TODO:
        // -make urgent and demand attention!

        this._promptWindow.set_demands_attention();

        this._adjustPromptPosition();
        this._monitorPromptWindowChanges();
        this._monitorLastSnapdWindowChanges();
    }

    _monitorLastSnapdWindowChanges() {
        this._lastFocusedSnapWindowChangesSignals?.destroy();
        this._lastFocusedSnapWindowChangesSignals = new SignalTracker.TransientSignalHolder(
            this._lastFocusedSnapWindowSignals);
        this._monitorWindowChanges(this._lastFocusedSnapWindow(),
            this._lastFocusedSnapWindowChangesSignals);
    }

    _monitorPromptWindowChanges() {
        this._promptWindowChangesSignals?.destroy();
        this._promptWindowChangesSignals = new SignalTracker.TransientSignalHolder(this);
        this._monitorWindowChanges(this._promptWindow, this._promptWindowChangesSignals);
    }

    _monitorWindowChanges(window, signalsTracker) {
        window.connectObject('position-changed',
            () => this._adjustPromptPosition(), signalsTracker);
        window.connectObject('size-changed',
            () => this._adjustPromptPosition(), signalsTracker);
    }

    _moveResizePromptWindow(x, y, width, height) {
        this._promptWindowChangesSignals?.destroy();
        this._promptWindow.move_resize_frame(
            true,
            x,
            y,
            width,
            height
        );
        this._monitorPromptWindowChanges();
    }

    _moveResizeLastFocusedSnapWindow(x, y, width, height) {
        const lastFocusedSnapWindow = this._lastFocusedSnapWindow();
        this._lastFocusedSnapWindowChangesSignals?.destroy();
        lastFocusedSnapWindow.move_resize_frame(
            true,
            x,
            y,
            width,
            height
        );
        this._monitorLastSnapdWindowChanges();
    }

    _adjustPromptPosition() {
        const lastFocusedSnapWindow = this._lastFocusedSnapWindow();
        const promptWindowFrameRect = this._promptWindow.get_frame_rect();
        const frameRect = lastFocusedSnapWindow.get_frame_rect();

        if (this._lastRect &&
            this._lastRect.width === promptWindowFrameRect.width &&
            this._lastRect.height === promptWindowFrameRect.height &&
            !lastFocusedSnapWindow.is_maximized()) {

            const newX = frameRect.x + (promptWindowFrameRect.x - this._lastRect.x);
            let newY = frameRect.y + (promptWindowFrameRect.y - this._lastRect.y);

            const workArea = lastFocusedSnapWindow.get_work_area_current_monitor();
            let isTopWorkArea = workArea.y === 0;

            if (!isTopWorkArea && frameRect.y <= 100) {
                const monitor = global.display.get_monitor_index_for_rect(workArea);
                const monitorGeometry = global.display.get_monitor_geometry(monitor);
                isTopWorkArea = monitorGeometry.y === 0;
            }

            if (isTopWorkArea && frameRect.y <= workArea.y &&
                promptWindowFrameRect.y <= this._lastRect.y) {
                newY = frameRect.y;

                const centerY = frameRect.y + frameRect.height / 2;
                const promptY = Math.round(centerY - promptWindowFrameRect.height / 2);
                promptWindowFrameRect.y = promptY;

                this._moveResizePromptWindow(
                    promptWindowFrameRect.x,
                    promptWindowFrameRect.y,
                    promptWindowFrameRect.width,
                    promptWindowFrameRect.height
                );
            }

            this._moveResizeLastFocusedSnapWindow(
                newX,
                newY,
                frameRect.width,
                frameRect.height
            );

            this._lastRect = promptWindowFrameRect;

            return;
        }

        const centerX = frameRect.x + frameRect.width / 2;
        const centerY = frameRect.y + frameRect.height / 2;

        const newX = Math.round(centerX - promptWindowFrameRect.width / 2);
        const newY = Math.round(centerY - promptWindowFrameRect.height / 2);

        this._moveResizePromptWindow(
            newX,
            newY,
            promptWindowFrameRect.width,
            promptWindowFrameRect.height
        );

        this._lastRect = {
            x: newX,
            y: newY,
            width: promptWindowFrameRect.width,
            height: promptWindowFrameRect.height,
        };
    }

    _lastFocusedSnapWindow() {
        const maxUserTime = Math.max(...this._snapWindows.map(w => w.get_user_time()));
        return this._snapWindows.find(w => w.get_user_time() === maxUserTime);
    }

    get promptWindow() {
        return this._promptWindow;
    }

    get promptApp() {
        return this._promptApp;
    }

    get snapApp() {
        return this._snapApp;
    }

    get snapWindows() {
        return this._snapWindows;
    }

    destroy() {
        this._injectionManager.clear();
        this._injectionManager = null;
        this._snapWindows.forEach(w => Main.wm._checkDimming(w));

        super.destroy();
    }
});
