imports.gi.versions.Adw = '1';
imports.gi.versions.Gtk = '4.0';
const { Adw, Gio, Gtk, GLib } = imports.gi;

Gio._promisify(Gio.DBusProxy.prototype, 'init_async');
Gio._promisify(Gio.DBusProxy.prototype, 'call');

class PermissionApp {
    constructor() {
        this.appPid = GLib.getenv("PROMPTING_APP_PID")
        this.snapAppID = GLib.getenv("PROMPTING_APP_SNAP_ID")

        if (!(this.appPid > 1) || !this.snapAppID)
            throw new Error("Missing PID or app ID")

        print(`Simulating Prompt request coming from ${this.snapAppID} with `+
            `PID ${this.appPid}`)

        this.application = new Adw.Application({
            application_id: 'com.canonical.Snapd.AppArmor.PromptingClient.Dialog',
            flags: Gio.ApplicationFlags.DEFAULT_FLAGS
        });

        this.application.connect('activate', () => this.on_activate());
    }

    on_activate() {
        // Create main window
        this.window = new Adw.PreferencesWindow({
            title: 'Permission Test',
            application: this.application,
            default_width: 600,
            default_height: 400,
        });

        const page = new Adw.PreferencesPage({
            title: 'Permissions',
            icon_name: 'dialog-password-symbolic'
        });

        const group = new Adw.PreferencesGroup({
            title: 'File Access',
            description: 'Control which files the application can access'
        });

        const switchRow = new Adw.SwitchRow({
            title: 'Allow reading /foo/bar/baz',
            subtitle: 'Required for core functionality',
            active: false
        });
        group.add(switchRow);

        const expandedGroup = new Adw.PreferencesGroup({
            title: 'Additional Options',
            description: 'Configure advanced permissions',
            visible: false
        });

        // Some fake content to make the dialog to grow on request.
        const row1 = new Adw.ActionRow({
            title: 'Read access',
            subtitle: 'Allow reading files in this directory'
        });
        expandedGroup.add(row1);

        const row2 = new Adw.ActionRow({
            title: 'Write access',
            subtitle: 'Allow writing files in this directory'
        });
        expandedGroup.add(row2);

        const row3 = new Adw.ActionRow({
            title: 'Execute access',
            subtitle: 'Allow executing files in this directory'
        });
        expandedGroup.add(row3);

        switchRow.connect('notify::active', () => {
            expandedGroup.visible = switchRow.active;

            if (switchRow.active) {
                this.window.default_height = 600;
                this.window.set_size_request(600, 600);
            } else {
                this.window.default_height = 400;
                this.window.set_size_request(600, 400);
            }
        });

        page.add(group);
        page.add(expandedGroup);
        this.window.add(page);

        this.window.present();

        // Move to top to do the request earlier...
        this.request_permission().catch(logError);
    }

    async request_permission() {
        try {
            // Create D-Bus proxy
            const proxy = new Gio.DBusProxy({
                gConnection: Gio.DBus.session,
                gName: 'com.canonical.Shell.PermissionPrompting',
                gObjectPath: '/com/canonical/Shell/PermissionPrompting',
                gInterfaceName: 'com.canonical.Shell.PermissionPrompting'
            });

            await proxy.init_async(GLib.PRIORITY_DEFAULT, null)
            await proxy.call(
                'Prompt',
                new GLib.Variant('(st)', [this.snapAppID, this.appPid]),
                Gio.DBusCallFlags.NONE,
                -1,
                null
            );
        } catch (e) {
            logError(e, 'Permission Request Error');
        }
    }
}

const app = new PermissionApp();
app.application.run(null);
