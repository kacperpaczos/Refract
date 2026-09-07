import {
    GLib,
    Gio,
    GObject,
    Shell,
} from './dependencies/gi.js';

import {SignalTracker} from './dependencies/shell/misc.js';

const {_gi: Gi} = imports;

export const SignalsHandlerFlags = Object.freeze({
    NONE: 0,
    CONNECT_AFTER: 1,
});

const GENERIC_KEY = Symbol('generic');

export const DestroyableGObject = GObject.registerClass({
    Signals: {'destroy': {}},
}, class DestroyableGObject extends GObject.Object {
    destroy() {
        this.emit('destroy');
    }
});

SignalTracker.registerDestroyableType(DestroyableGObject);

/**
 * Simplify global signals and function injections handling
 * abstract class
 */
const BasicHandler = class DashToDockBasicHandler {
    static get genericKey() {
        return GENERIC_KEY;
    }

    constructor(parentObject) {
        this._storage = Object.create(null);

        if (parentObject) {
            if (!(parentObject.connect instanceof Function))
                throw new TypeError('Not a valid parent object');

            if (!(parentObject instanceof GObject.Object) ||
                GObject.signal_lookup('destroy', parentObject.constructor.$gtype)) {
                this._parentObject = parentObject;
                this._destroyId = parentObject.connect('destroy', () => this.destroy());
            }
        }
    }

    add(...args) {
        // Convert arguments object to array, concatenate with generic
        // Call addWithLabel with ags as if they were passed arguments
        this.addWithLabel(GENERIC_KEY, ...args);
    }

    clear() {
        Object.getOwnPropertySymbols(this._storage).forEach(label =>
            this.removeWithLabel(label));
    }

    destroy() {
        this._parentObject?.disconnect(this._destroyId);
        this._parentObject = null;

        this.clear();
    }

    block() {
        Object.getOwnPropertySymbols(this._storage).forEach(label =>
            this.blockWithLabel(label));
    }

    unblock() {
        Object.getOwnPropertySymbols(this._storage).forEach(label =>
            this.unblockWithLabel(label));
    }

    addWithLabel(label, ...args) {
        if (typeof label !== 'symbol')
            throw new Error(`Invalid label ${label}, must be a symbol`);

        let argsArray = [...args];
        if (argsArray.every(arg => !Array.isArray(arg)))
            argsArray = [argsArray];

        if (this._storage[label] === undefined)
            this._storage[label] = [];

        // Skip first element of the arguments
        for (const argArray of argsArray) {
            if (argArray.length < 3)
                throw new Error('Unexpected number of arguments');
            const item = this._storage[label];
            try {
                item.push(this._create(...argArray));
            } catch (e) {
                logError(e);
            }
        }
    }

    removeWithLabel(label) {
        this._storage[label]?.reverse().forEach(item => this._remove(item));
        delete this._storage[label];
    }

    blockWithLabel(label) {
        (this._storage[label] || []).forEach(item => this._block(item));
    }

    unblockWithLabel(label) {
        (this._storage[label] || []).forEach(item => this._unblock(item));
    }

    _removeByItem(item) {
        Object.getOwnPropertySymbols(this._storage).forEach(label =>
            (this._storage[label] = this._storage[label].filter(it => {
                if (!this._itemsEqual(it, item))
                    return true;
                this._remove(item);
                return false;
            })));
    }

    // Virtual methods to be implemented by subclass

    /**
     * Create single element to be stored in the storage structure
     *
     * @param _object
     * @param _element
     * @param _callback
     */
    _create(_object, _element, _callback) {
        throw new GObject.NotImplementedError(`_create in ${this.constructor.name}`);
    }

    /**
     * Correctly delete single element
     *
     * @param _item
     */
    _remove(_item) {
        throw new GObject.NotImplementedError(`_remove in ${this.constructor.name}`);
    }

    /**
     * Block single element
     *
     * @param _item
     */
    _block(_item) {
        throw new GObject.NotImplementedError(`_block in ${this.constructor.name}`);
    }

    /**
     * Unblock single element
     *
     * @param _item
     */
    _unblock(_item) {
        throw new GObject.NotImplementedError(`_unblock in ${this.constructor.name}`);
    }

    _itemsEqual(itemA, itemB) {
        if (itemA === itemB)
            return true;

        if (itemA.length !== itemB.length)
            return false;

        return itemA.every((_, idx) => itemA[idx] === itemB[idx]);
    }
};

/**
 * Manage global signals
 */
export class GlobalSignalsHandler extends BasicHandler {
    _create(object, event, callback, flags = SignalsHandlerFlags.NONE) {
        if (!object)
            throw new Error('Impossible to connect to an invalid object');

        const after = flags === SignalsHandlerFlags.CONNECT_AFTER;
        const connector = after ? object.connect_after : object.connect;

        if (!connector) {
            throw new Error(`Requested to connect to signal '${event}', ` +
                `but no implementation for 'connect${after ? '_after' : ''}' ` +
                `found in ${object.constructor.name}`);
        }

        const item = [object];
        const isDestroy = event === 'destroy';
        const isParentObject = object === this._parentObject;

        if (isDestroy && !isParentObject) {
            const originalCallback = callback;
            callback = () => {
                this._removeByItem(item);
                originalCallback();
            };
        }
        const id = connector.call(object, event, callback);
        item.push(id);

        if (isDestroy && isParentObject) {
            this._parentObject.disconnect(this._destroyId);
            this._destroyId =
                this._parentObject.connect('destroy', () => this.destroy());
        }

        return item;
    }

    _remove(item) {
        const [object, id] = item;
        object.disconnect(id);
    }

    _block(item) {
        const [object, id] = item;

        if (object instanceof GObject.Object)
            GObject.Object.prototype.block_signal_handler.call(object, id);
    }

    _unblock(item) {
        const [object, id] = item;

        if (object instanceof GObject.Object)
            GObject.Object.prototype.unblock_signal_handler.call(object, id);
    }
}

/**
 * Manage function injection: both instances and prototype can be overridden
 * and restored
 */
export class InjectionsHandler extends BasicHandler {
    _create(object, name, injectedFunction) {
        const original = object[name];

        if (!(original instanceof Function))
            throw new Error(`Function ${name}() is not available for ${object}`);

        object[name] = function (...args) {
            return injectedFunction.call(this, original, ...args);
        };
        return [object, name, original];
    }

    _remove(item) {
        const [object, name, original] = item;
        object[name] = original;
    }
}

/**
 * Manage vfunction injection: both instances and prototype can be overridden
 * and restored
 */
export class VFuncInjectionsHandler extends BasicHandler {
    _create(prototype, name, injectedFunction) {
        const original = prototype[`vfunc_${name}`];
        if (!(original instanceof Function))
            throw new Error(`Virtual function ${name} is not available for ${prototype}`);
        this._replaceVFunc(prototype, name, injectedFunction);
        return [prototype, name];
    }

    _remove(item) {
        const [prototype, name] = item;
        const originalVFunc = prototype[`vfunc_${name}`];
        try {
            // This may fail if trying to reset to a never-overridden vfunc
            // as gjs doesn't consider it a function, even if it's true that
            // originalVFunc instanceof Function.
            this._replaceVFunc(prototype, name, originalVFunc);
        } catch {
            try {
                this._replaceVFunc(prototype, name, function (...args) {
                    // eslint-disable-next-line no-invalid-this
                    return originalVFunc.call(this, ...args);
                });
            } catch (e) {
                logError(e, `Removing vfunc_${name}`);
            }
        }
    }

    _replaceVFunc(prototype, name, func) {
        if (Gi.gobject_prototype_symbol && Gi.gobject_prototype_symbol in prototype)
            prototype = prototype[Gi.gobject_prototype_symbol];

        return prototype[Gi.hook_up_vfunc_symbol](name, func);
    }
}

/**
 * Manage properties injection: both instances and prototype can be overridden
 * and restored
 */
export class PropertyInjectionsHandler extends BasicHandler {
    constructor(parentObject, params) {
        super(parentObject);
        this._params = params;
    }

    _create(instance, name, injectedPropertyDescriptor) {
        if (!this._params?.allowNewProperty && !(name in instance))
            throw new Error(`Object ${instance} has no '${name}' property`);

        const {prototype} = instance.constructor;
        const originalPropertyDescriptor = Object.getOwnPropertyDescriptor(prototype, name) ??
            Object.getOwnPropertyDescriptor(instance, name);

        Object.defineProperty(instance, name, {
            ...originalPropertyDescriptor,
            ...injectedPropertyDescriptor,
            ...{configurable: true},
        });
        return [instance, name, originalPropertyDescriptor];
    }

    _remove(item) {
        const [instance, name, originalPropertyDescriptor] = item;
        if (originalPropertyDescriptor)
            Object.defineProperty(instance, name, originalPropertyDescriptor);
        else
            delete instance[name];
    }
}

/**
 * Construct a map of gtk application window object paths to MetaWindows.
 */
export function getWindowsByObjectPath() {
    const windowsByObjectPath = new Map();
    const {workspaceManager} = global;
    const workspaces = [...new Array(workspaceManager.nWorkspaces)].map(
        (_c, i) => workspaceManager.get_workspace_by_index(i));

    workspaces.forEach(ws => {
        ws.list_windows().forEach(w => {
            const path = w.get_gtk_window_object_path();
            if (path)
                windowsByObjectPath.set(path, w);
        });
    });

    return windowsByObjectPath;
}

export function getWindowsByBusName() {
    const windowsByObjectPath = new Map();
    const {workspaceManager} = global;
    const workspaces = [...new Array(workspaceManager.nWorkspaces)].map(
        (_c, i) => workspaceManager.get_workspace_by_index(i));

    workspaces.forEach(ws => {
        ws.list_windows().forEach(w => {
            const busName = w.get_gtk_unique_bus_name();
            if (busName)
                windowsByObjectPath.set(busName, w);
        });
    });

    return windowsByObjectPath;
}

export function getWindowByBusName(busName) {
    return global.display.list_all_windows().find(w =>
        w.get_gtk_unique_bus_name() === busName);
}

/**
 * Re-implements shell_app_compare so that can be used to resort running apps
 *
 * @param appA
 * @param appB
 */
export function shellAppCompare(appA, appB) {
    if (appA.state !== appB.state) {
        if (appA.state === Shell.AppState.RUNNING)
            return -1;
        return 1;
    }

    const windowsA = appA.get_windows();
    const windowsB = appB.get_windows();

    const isMinimized = windows => !windows.some(w => w.showing_on_its_workspace());
    const minimizedB = isMinimized(windowsB);
    if (isMinimized(windowsA) !== minimizedB) {
        if (minimizedB)
            return -1;
        return 1;
    }

    if (appA.state === Shell.AppState.RUNNING) {
        if (windowsA.length && !windowsB.length)
            return -1;
        else if (!windowsA.length && windowsB.length)
            return 1;

        const lastUserTime = windows =>
            Math.max(...windows.map(w => w.get_user_time()));
        return lastUserTime(windowsB) - lastUserTime(windowsA);
    }

    return 0;
}

/**
 * Re-implements shell_app_compare_windows
 *
 * @param winA
 * @param winB
 */
export function shellWindowsCompare(winA, winB) {
    const activeWorkspace = global.workspaceManager.get_active_workspace();
    const wsA = winA.get_workspace() === activeWorkspace;
    const wsB = winB.get_workspace() === activeWorkspace;

    if (wsA && !wsB)
        return -1;
    else if (!wsA && wsB)
        return 1;

    const visA = winA.showing_on_its_workspace();
    const visB = winB.showing_on_its_workspace();

    if (visA && !visB)
        return -1;
    else if (!visA && visB)
        return 1;

    return winB.get_user_time() - winA.get_user_time();
}

export const CancellableChild = GObject.registerClass({
    Properties: {
        'parent': GObject.ParamSpec.object(
            'parent', 'parent', 'parent',
            GObject.ParamFlags.READWRITE | GObject.ParamFlags.CONSTRUCT_ONLY,
            Gio.Cancellable.$gtype),
    },
},
class CancellableChild extends Gio.Cancellable {
    _init(parent) {
        if (parent && !(parent instanceof Gio.Cancellable))
            throw TypeError('Not a valid cancellable');

        super._init({parent});

        if (parent?.is_cancelled()) {
            this.cancel();
            return;
        }

        this._connectToParent();
    }

    _connectToParent() {
        this._connectId = this.parent?.connect(() => {
            this._realCancel();

            if (this._disconnectIdle)
                return;

            this._disconnectIdle = GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
                delete this._disconnectIdle;
                this._disconnectFromParent();
                return GLib.SOURCE_REMOVE;
            });
        });
    }

    _disconnectFromParent() {
        if (this._connectId && !this._disconnectIdle) {
            this.parent.disconnect(this._connectId);
            delete this._connectId;
        }
    }

    _realCancel() {
        Gio.Cancellable.prototype.cancel.call(this);
    }

    cancel() {
        this._disconnectFromParent();
        this._realCancel();
    }
});

export async function getSenderPid(sender) {
    const res = await Gio.DBus.session.call(
        'org.freedesktop.DBus',
        '/',
        'org.freedesktop.DBus',
        'GetConnectionUnixProcessID',
        new GLib.Variant('(s)', [sender]),
        new GLib.VariantType('(u)'),
        Gio.DBusCallFlags.NONE,
        -1,
        null);
    const [pid] = res.deepUnpack();
    return pid;
}
