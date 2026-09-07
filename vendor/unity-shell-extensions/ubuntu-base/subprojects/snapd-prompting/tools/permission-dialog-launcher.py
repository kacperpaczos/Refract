import os
import gi
import subprocess
import sys

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import GLib, Gtk, Adw

AA_RUNNER_NAME = "run-with-aa-profile.py"
AA_RUNNER_PATH = os.path.join(os.path.dirname(__file__), AA_RUNNER_NAME)

PERMISSION_DIALOG_NAME = "permission-dialog.js"
PERMISSION_DIALOG_PATH = os.path.join(os.path.realpath(os.path.dirname(__file__)), PERMISSION_DIALOG_NAME)

# This assumes that these snaps are installed and they use "classic" confinement
# otherwise other custom AppArmor profiles should be used instead.
PROMPTING_APP_SNAP_ID = "snap.code.code"
PERMISSION_DIALOG_SNAP_ID = "snap.snapcraft.snapcraft"

def snap_desktop_id_from_app_id(app_id):
    return app_id[len("snap."):].replace(".", "_")

class PromptTrigger(Adw.Application):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.connect('activate', self.on_activate)

    def on_activate(self, _):
        self.window = Adw.ApplicationWindow(application=self)
        self.window.set_default_size(800, 600)
        # self.window.maximize()

        win_content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.window.set_content(win_content_box)

        # Disable this to keep it fixed-size.
        header_bar = Adw.HeaderBar()
        header_bar.set_title_widget(Gtk.Label(label="App Prompting Example"))
        header_bar.set_show_end_title_buttons(True)
        win_content_box.append(header_bar)

        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        vbox.set_margin_top(12)
        vbox.set_margin_bottom(12)
        vbox.set_margin_start(12)
        vbox.set_margin_end(12)
        win_content_box.append(vbox)

        avatar = Adw.Avatar(size=128, text="Prompting App", show_initials=True)
        vbox.append(avatar)

        info_msg = Gtk.Label()
        info_msg.set_markup("<b>Believe me: I AM VS CODE</b>")
        info_msg.set_margin_bottom(32)
        vbox.append(info_msg)

        button = self.make_test_button(label="Launch Prompting Request!")
        vbox.append(button)

        button = self.make_test_button(label="Multi-window Prompting request")
        button.set_sensitive(False)
        vbox.append(button)

        button = self.make_test_button(label="Multi-window (different PIDs) Prompting request")
        button.set_sensitive(False)
        vbox.append(button)

        button = self.make_test_button(label="Windows-less Prompting request")
        button.set_sensitive(False)
        vbox.append(button)

        self.status = Gtk.Label()
        vbox.append(self.status)

        self.window.present()

    def make_test_button(self, label, launch_flags=[]):
        button = Gtk.Button(label=label)
        button.connect("clicked", self.on_launch_button_clicked)
        button.set_margin_top(6)
        button.set_margin_bottom(6)
        button.set_margin_start(8)
        button.set_margin_end(8)
        return button

    def on_launch_button_clicked(self, button):
        # Disable button during execution
        button.set_sensitive(False)
        self.status.set_label("It's prompting time!")

        GLib.timeout_add(100, self.launch_blocking_app, button)

    def launch_blocking_app(self, button):
        self.current_button = button
        app_id = self.get_application_id()
        pid = os.getpid()

        prompting_snap_id = PROMPTING_APP_SNAP_ID
        command = [
            "env",
            f"AA_PROFILE={PERMISSION_DIALOG_SNAP_ID}",
            f"PROMPTING_APP_PID={os.getpid()}",
            f"PROMPTING_APP_SNAP_ID={snap_desktop_id_from_app_id(prompting_snap_id)}",
            sys.executable,
            AA_RUNNER_PATH,
            "gjs",
            PERMISSION_DIALOG_PATH,
            app_id, str(pid),
        ]

        try:
            p = subprocess.Popen(command,
                                 stdin=sys.stdin,
                                 stdout=sys.stdout,
                                 stderr=sys.stderr)
            # YAY, let's hang!
            p.wait()

        except Exception as e:
            self.status.set_label(f"Error: {str(e)}")
            print(e)
        finally:
            # Re-enable button after completion
            self.current_button.set_sensitive(True)

if __name__ == "__main__":
    with open("/proc/self/attr/current", "rt", encoding="utf-8") as attr:
        if attr.read().strip() == "unconfined":
            os.environ["AA_PROFILE"] = PROMPTING_APP_SNAP_ID
            os.execv(sys.executable, [sys.executable, AA_RUNNER_PATH, sys.executable] + sys.argv)

    app = PromptTrigger(application_id="com.canonical.PromptTrigger")
    app.run()
