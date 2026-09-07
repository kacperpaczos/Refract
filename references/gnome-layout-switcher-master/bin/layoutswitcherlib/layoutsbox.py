import sys
import subprocess
import shutil
import re
#import os # unused import
import os
import tempfile
import atexit

from pathlib import Path
from glob import glob

import gi
import click

#from gi.repository import Gtk, Gdk, GObject, GLib, Gio # Gobject & Gio unused
from gi.repository import Gtk, Gdk, GLib
gi.require_version("Gtk", "3.0")

from .config import UserConf

# Define the path to css file
css_file = Path("~/.config/gtk-3.0/gtk.css").expanduser()

# Define a temp folder to store preview
temp_dir = tempfile.mkdtemp()

# Define asset in use
asset = ["manjaro-gdm-branding"]

nvidia_present = subprocess.run(
    'lspci -v | grep -q "Kernel driver in use: nvidia"', shell=True, check=False).returncode == 0


class Opacity:
    TOP = 1
    MIDDLE = 0.9
    LOW = 0.7


def rm_tmp_dir():
    shutil.rmtree(temp_dir, ignore_errors=True)


atexit.register(rm_tmp_dir)

@click.command(help="Apply traditional layout")
def apply_traditional():
    enabled = subprocess.getoutput('gsettings get org.gnome.shell enabled-extensions')
    required_extensions = (
        'dash-to-panel@jderose9.github.com',
        'arcmenu@arcmenu.com',
        'appindicatorsupport@rgcjonas.gmail.com',
        'gtk4-ding@smedius.gitlab.com',
        'gnome-ui-tune@itstime.tech'
        )
    conflicting_extensions = (
        'dash-to-dock@micxgx.gmail.com',
        'unite@hardpixel.eu',
        'places-menu@gnome-shell-extensions.gcampax.github.com',
        'material-shell@papyelgringo',
        'no-overview@fthx',
        'vertical-overview@RensAlthuis.github.com',
        'window-list@gnome-shell-extensions.gcampax.github.com',
        'forge@jmmaranan.com',
        'space-bar@luchrioh'
        )

    subprocess.run('gsettings set org.gnome.shell.extensions.dash-to-panel panel-element-positions \'{"0":[{"element":"showAppsButton","visible":false,"position":"stackedTL"},{"element":"activitiesButton","visible":false,"position":"stackedTL"},{"element":"leftBox","visible":true,"position":"stackedTL"},{"element":"taskbar","visible":true,"position":"stackedTL"},{"element":"centerBox","visible":true,"position":"stackedBR"},{"element":"rightBox","visible":true,"position":"stackedBR"},{"element":"dateMenu","visible":true,"position":"stackedBR"},{"element":"systemMenu","visible":true,"position":"stackedBR"},{"element":"desktopButton","visible":true,"position":"stackedBR"}]}\';\
                gsettings set org.gnome.shell.extensions.dash-to-panel panel-position BOTTOM;\
                gsettings set org.gnome.shell.extensions.dash-to-panel show-running-apps true;\
                gsettings set org.gnome.shell.extensions.dash-to-panel panel-size 48;\
                gsettings set org.gnome.shell.extensions.arcmenu custom-menu-button-icon-size 32.0;\
                gsettings set org.gnome.shell.extensions.arcmenu menu-button-appearance Icon;\
                gsettings set org.gnome.shell.extensions.arcmenu dash-to-panel-standalone false;\
                gsettings set org.gnome.shell.extensions.arcmenu menu-layout Default;\
                gsettings set org.gnome.shell.extensions.arcmenu menu-button-icon "Distro_Icon";\
                gsettings set org.gnome.shell.extensions.arcmenu distro-icon 3;\
                gsettings set org.gnome.shell.extensions.arcmenu hide-overview-on-startup true;\
                gsettings set org.gnome.desktop.wm.preferences button-layout ":minimize,maximize,close"', shell=True, check=False)
    for ext in conflicting_extensions:
        if ext in enabled:
            GLib.spawn_command_line_sync(f'gnome-extensions disable {ext}')
            print(f"disabled {ext}")
    for ext in required_extensions:
        if ext not in enabled:
            GLib.spawn_command_line_sync(f'gnome-extensions enable {ext}')
            print(f"enabled {ext}")
    save_layout_conf("traditional")

@click.command(help="Apply manjaro layout")
def apply_manjaro():
    enabled = subprocess.getoutput('gsettings get org.gnome.shell enabled-extensions')
    required_extensions = (
        'dash-to-dock@micxgx.gmail.com',
        'appindicatorsupport@rgcjonas.gmail.com',
        'gnome-ui-tune@itstime.tech'
        )
    conflicting_extensions = (
        'arcmenu@arcmenu.com',
        'dash-to-panel@jderose9.github.com',
        'places-menu@gnome-shell-extensions.gcampax.github.com',
        'material-shell@papyelgringo',
        'window-list@gnome-shell-extensions.gcampax.github.com',
        'appindicatorsupport@rgcjonas.gmail.com',
        'no-overview@fthx',
        'forge@jmmaranan.com',
        'space-bar@luchrioh'
        )

    subprocess.run('gsettings set org.gnome.shell.extensions.dash-to-dock dock-position BOTTOM;\
                gsettings set org.gnome.shell.extensions.dash-to-dock extend-height false;\
                gsettings set org.gnome.shell.extensions.dash-to-dock dock-fixed false', shell=True, check=False)
    for ext in conflicting_extensions:
        if ext in enabled:
            GLib.spawn_command_line_sync(f'gnome-extensions disable {ext}')
            print(f"disabled {ext}")
    for ext in required_extensions:
        if ext not in enabled:
            GLib.spawn_command_line_sync(f'gnome-extensions enable {ext}')
            print(f"enabled {ext}")
    save_layout_conf("manjaro")

@click.command(help="Apply gnome layout")
def apply_gnome():
    enabled = subprocess.getoutput('gsettings get org.gnome.shell enabled-extensions')
    conflicting_extensions = (
        'dash-to-dock@micxgx.gmail.com',
        'arcmenu@arcmenu.com',
        'gtk4-ding@smedius.gitlab.com',
        'dash-to-panel@jderose9.github.com',
        'places-menu@gnome-shell-extensions.gcampax.github.com',
        'material-shell@papyelgringo',
        'window-list@gnome-shell-extensions.gcampax.github.com',
        'appindicatorsupport@rgcjonas.gmail.com',
        'no-overview@fthx',
        'gnome-ui-tune@itstime.tech',
        'forge@jmmaranan.com',
        'space-bar@luchrioh'
        )

    GLib.spawn_command_line_sync(
        'gsettings set org.gnome.desktop.wm.preferences button-layout ":minimize,maximize,close"')

    for ext in conflicting_extensions:
        if ext in enabled:
            GLib.spawn_command_line_sync(f'gnome-extensions disable {ext}')
            print(f"disabled {ext}")
    save_layout_conf("gnome")

@click.command(help="Apply tiling layout")
def apply_tiling():
    enabled = subprocess.getoutput('gsettings get org.gnome.shell enabled-extensions')
    required_extensions = (
        'appindicatorsupport@rgcjonas.gmail.com',
        'arcmenu@arcmenu.com',
        'forge@jmmaranan.com',
        'gnome-ui-tune@itstime.tech',
        'space-bar@luchrioh'
        )
    conflicting_extensions = (
        'dash-to-panel@jderose9.github.com',
        'material-shell@papyelgringo',
        'pop-shell@system76.com',
        'workspace-indicator@gnome-shell-extensions.gcampax.github.com'
        )

    subprocess.run('gsettings set org.gnome.shell.extensions.arcmenu menu-layout GnomeOverview;\
                gsettings set org.gnome.shell.extensions.arcmenu menu-button-icon "Distro_Icon";\
                gsettings set org.gnome.shell.extensions.arcmenu distro-icon 3;\
                gsettings set org.gnome.shell.extensions.arcmenu custom-menu-button-icon-size 20;\
                gsettings set org.gnome.shell.extensions.space-bar.shortcuts open-menu "@as []"', shell=True, check=False)

    GLib.spawn_command_line_sync(
        'gsettings set org.gnome.desktop.wm.preferences button-layout ":minimize,maximize,close"')
    for ext in conflicting_extensions:
        if ext in enabled:
            GLib.spawn_command_line_sync(f'gnome-extensions disable {ext}')
            print(f"disabled {ext}")
    for ext in required_extensions:
        if ext not in enabled:
            GLib.spawn_command_line_sync(f'gnome-extensions enable {ext}')
            print(f"enabled {ext}")
    save_layout_conf("tiling")

def get_layouts():
    return ({"id": "traditional", "label": "Traditional", "x": 3, "y": 0},
            {"id": "manjaro", "label": "Manjaro", "x": 2, "y": 0},
            {"id": "tiling", "label": "Tiling", "x": 2, "y": 3},
            {"id": "gnome", "label": "GNOME", "x": 3, "y": 3},)

@click.command(help="Reload gnome shell")
def reload_gnome_shell():
    running_wayland = subprocess.run("pgrep Xwayland", shell=True, check=False)
    print(running_wayland)
    if running_wayland.returncode == 0:
        GLib.spawn_command_line_sync("gnome-session-quit --logout")
    else:
        GLib.spawn_command_line_sync(
            "busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell Eval s \'Meta.restart(\"Restarting GNOME...\")\'")


def replace_in_file(file_name: str, regex: str, value: str):
    """ replace all word in file """
    pattern = re.compile(regex)
    file_old = f"{file_name}~"
    shutil.copy(file_name, file_old)
    with open(file_old, "tr", encoding="utf-8") as file_read, open(file_name, "tw", encoding="utf-8") as file_write:
        # reading lines takes less ram that reading the entire file, but is also slower.
        for line in file_read:
            file_write.write(re.sub(pattern, value, line))
    Path(file_old).unlink()

def save_layout_conf(name):
    with UserConf() as conf:
        conf.write({"layout": name})
        print("Layout applied")

def shell(commands) -> tuple:
    """return true if command return 0 AND error message"""
    if "--dev" in sys.argv:
        print("Debug mode on:")
        print("Simulating shell commands:")
        print(f"{commands}")
    else:
        try:
            # only run if not in dev mode
            if isinstance(commands, str):
                # if is string then we only have one shell command
                subprocess.run(commands, text=True, shell=True, check=True)
            else:
                # is a tuple or list
                for cmd in commands:
                    subprocess.run(cmd, text=True, shell=True, check=True)
        except subprocess.CalledProcessError as err:
            return False, str(err)
    return True, ""

def get_extensions(chosen_layout):
    # List needed extensions
    extensions = {
        "manjaro": (
            "dash-to-dock@micxgx.gmail.com",
            "user-theme@gnome-shell-extensions.gcampax.github.com",
            "gnome-ui-tune@itstime.tech"
        ),
        "traditional": (
            "dash-to-panel@jderose9.github.com",
            "appindicatorsupport@rgcjonas.gmail.com",
            "arcmenu@arcmenu.com",
            "gnome-ui-tune@itstime.tech"
        ),
        "tiling": (
            "appindicatorsupport@rgcjonas.gmail.com",
            "arcmenu@arcmenu.com",
            "forge@jmmaranan.com",
            "gnome-ui-tune@itstime.tech",
            "space-bar@luchrioh"
        ),
        "gnome": (
        )
    }
    # List needed extension packages
    ext_pkgs = {
        "manjaro": ["gnome-shell-extension-dash-to-dock",
                    "gnome-shell-extension-gnome-ui-tune",
                    "gnome-shell-extension-appindicator"],
        "traditional": ["gnome-shell-extension-dash-to-panel",
                        "gnome-shell-extension-appindicator",
                        "gnome-shell-extension-arc-menu",
                        "gnome-shell-extension-gnome-ui-tune"],
        "tiling": ["gnome-shell-extension-appindicator",
                   "gnome-shell-extension-arc-menu",
                   "gnome-shell-extension-forge",
                   "gnome-shell-extension-gnome-ui-tune",
                   "gnome-shell-extension-space-bar"],
        "gnome": [""]
    }

    # Check if extensions are missing
    pkgs_missing = False
    try:
        for ext in extensions.get(chosen_layout):
            user_path = Path(f"~/.local/share/gnome-shell/extensions/{ext}").expanduser()
            sys_path = Path(f"/usr/share/gnome-shell/extensions/{ext}")
            if not Path.is_dir(sys_path) and not Path.is_dir(Path(user_path).expanduser()):
                pkgs_missing = True
            else:
                print("Already installed", ext)
    except (TypeError, AttributeError) as e:
        print(e)

    # Install missing packages with pamac
    if pkgs_missing:
        pkg_list = []
        for pkg in ext_pkgs.get(chosen_layout):
            pkg_installed = subprocess.run(f"pacman -Qq {pkg}&>/dev/null", shell=True, check=False)
            if pkg_installed.returncode == 1:
                pkg_list.append(pkg)
        print(f"Needed packages: {' '.join(pkg_list)}")
        shell(f"pamac-installer {' '.join(pkg_list)}")


# ----------------- branding functions --------------------------
def do_branding(remove: bool) -> tuple:
    arguments = asset
    if not isinstance(asset, str):
        arguments = " ".join(asset)
    if remove:
        commands = f"pamac-installer --remove  {arguments}"
    else:
        commands = f"pamac-installer  {arguments}"
    return shell(commands)


def get_asset_state() -> bool:
    arguments = asset
    if not isinstance(asset, str):
        arguments = " ".join(asset)
    asset_state = subprocess.run(f"pacman -Qq {arguments} > /dev/null 2>&1", shell=True, check=False)
    if asset_state.returncode == 0:
        return True
    return False
# ----------------- end branding functions ----------------------


class LayoutBox(Gtk.Box):

    def __init__(
        self, window: Gtk.Window, orientation=Gtk.Orientation.VERTICAL, spacing=1, usehello=False):
        super().__init__(orientation=orientation, spacing=spacing, expand=True)
        self.set_margin_top(16)
        """ initialize main box """
        self.layout = "traditional"
        self.window = window
        self.usehello = usehello  # if we want some diff in hello or standalone app...

        # PEP: no variable declaration outside __init__
        self.btn_layout_first = None
        self.branding_handler_id = None
        self.branding_active = None
        self.pop_id = None

        with UserConf() as conf:
            self.layout = conf.read("layout", "manjaro")
            print("current layout:", self.layout)

        self.previews = {}
        self.color_button = None
        self._current_color = None

        # self.set_default_size(300, 300)

        # Determine preview color and highllight color
        rgba = self.window.get_style_context().lookup_color("theme_fg_color").color
        self.default_color = "#{:02x}{:02x}{:02x}".format(*[int(c * 255) for c in rgba]).upper()

        rgba = self.window.get_style_context().lookup_color("theme_selected_bg_color").color
        self.highlight_color = "#{:02x}{:02x}{:02x}".format(*[int(c * 255) for c in rgba]).upper()

        # Stack settings
        stack = Gtk.Stack()
        stack.set_transition_type(Gtk.StackTransitionType.SLIDE_LEFT_RIGHT)
        stack.set_transition_duration(300)
        stack.set_hhomogeneous(False)
        stack.set_vhomogeneous(False)
        stack.set_vexpand(True)
        stack.props.valign = Gtk.Align.START
        self.create_page_layout(stack)
        self.create_page_theme(stack)
        self.current_color = ""  # set colors from .css
        self.show_all()
        dirty_hack = self.layout
        self.layout = "traditional"
        self.previews[self.layout].get_parent().btn.set_active(True)
        try:
            self.previews[dirty_hack].get_parent().btn.set_active(True)
        except KeyError:
            print(f'Chosen layout {dirty_hack} is not recognized, using the default instead')


    def create_page_layout(self, stack):
        """ Layout menu """
        vbox = Gtk.Grid(row_homogeneous=False,
                        column_homogeneous=False,
                        row_spacing=0,
                        margin_left=0,
                        margin_right=0,
                        margin_bottom=0,
                        margin_top=0)
        self.add(vbox)
        vbox.attach(stack, 1, 1, 1, 3)
        radiobox = Gtk.Grid(column_spacing=45,
                            row_homogeneous=False,
                            row_spacing=20,
                            margin_left=10,
                            margin_right=10,
                            margin_bottom=0,
                            margin_top=15)
        radiobox.set_hexpand(True)
        radiobox.props.halign = Gtk.Align.CENTER
        for layout in get_layouts():
            self.create_layout_btn(layout=layout, the_grid=radiobox)


        stack.add_titled(radiobox, "radiobox", "Layout")
        stack.props.margin_bottom = 0
        stack_switcher = Gtk.StackSwitcher()
        stack_switcher.set_stack(stack)
        stack_switcher.set_hexpand(True)
        stack_switcher.props.halign = Gtk.Align.CENTER
        vbox.attach(stack_switcher, 1, 0, 1, 1)

        applybutton = Gtk.Button.new_with_label("Apply")
        applybutton.connect("clicked", self.on_layoutapply_clicked)
        radiobox.attach(applybutton, 3, 6, 1, 1)
        applybutton.props.valign = Gtk.Align.END

        reloadbutton = Gtk.Button.new_with_label("Reload Desktop")
        reloadbutton.connect("clicked", self.on_reload_clicked)
        radiobox.attach(reloadbutton, 2, 6, 1, 1)
        reloadbutton.props.valign = Gtk.Align.END

    def create_page_theme(self, stack):
        """ The theme tab """
        theme_grid = Gtk.Grid(
            row_spacing=15,
            column_spacing=50,
            margin_left=40,
            margin_right=40,
            margin_bottom=0,
            margin_top=30)
        theme_grid.set_hexpand(False)
        theme_grid.props.valign = Gtk.Align.CENTER
        theme_grid.props.halign = Gtk.Align.CENTER
        theme_grid.props.row_homogeneous = False
        theme_grid.props.column_homogeneous = False
        # theme_grid.props.column_spacing = 30
        theme_grid.set_vexpand(False)
        stack.add_titled(theme_grid, "theme_grid", "Settings")

        # ----------------------------------------------
        # Manjaro branding toggle
        manjaro_switch = Gtk.Switch()
        manjaro_switch.props.valign = Gtk.Align.CENTER
        manjaro_switch.props.halign = Gtk.Align.CENTER

        # get branding state True/False
        self.branding_active = get_asset_state()
        manjaro_switch.set_active(self.branding_active)
        self.branding_handler_id = manjaro_switch.connect(
            "notify::active", self.on_branding_activated)
        # --- branding initialize end
        # ----------------------------------------------

        manjaro_label = Gtk.Label()
        manjaro_label.set_markup("Manjaro branding")
        manjaro_label.props.halign = Gtk.Align.START

        # System tray
        tray_switch = Gtk.Switch()
        tray_switch.props.valign = Gtk.Align.CENTER
        tray_switch.props.halign = Gtk.Align.CENTER
        tray_enabled = subprocess.run(
            "gnome-extensions info appindicatorsupport@rgcjonas.gmail.com | grep -q ENABLED", shell=True, check=False)
        if tray_enabled.returncode == 0:
            tray_switch.set_active(True)
        else:
            tray_switch.set_active(False)
        tray_switch.connect("notify::active", self.on_tray_activated)
        tray_label = Gtk.Label()
        tray_label.set_markup("System tray")
        tray_label.props.halign = Gtk.Align.START

        # Desktop icons
        desk_switch = Gtk.Switch()
        desk_switch.props.valign = Gtk.Align.CENTER
        desk_switch.props.halign = Gtk.Align.CENTER
        desk_enabled = subprocess.run(
            "gnome-extensions info gtk4-ding@smedius.gitlab.com | grep -q ENABLED", shell=True, check=False)
        if desk_enabled.returncode == 0:
            desk_switch.set_active(True)
        else:
            desk_switch.set_active(False)
        desk_switch.connect("notify::active", self.on_desk_activated)
        desk_label = Gtk.Label()
        desk_label.set_markup("Desktop icons")
        desk_label.props.halign = Gtk.Align.START

        # GNOME Tweaks
        tweaks_button = Gtk.Button.new_with_label("Open")
        tweaks_button.connect("clicked", self.on_gnometweaks_activated)
        tweaks_button.props.valign = Gtk.Align.CENTER
        tweaks_button.props.halign = Gtk.Align.CENTER
        tweaks_label = Gtk.Label()
        tweaks_label.set_markup("GNOME Tweaks")
        tweaks_label.props.halign = Gtk.Align.START

        # GNOME Extensions
        ext_button = Gtk.Button.new_with_label("Open")
        ext_button.connect("clicked", self.on_gnomext_activated)
        ext_button.props.valign = Gtk.Align.CENTER
        ext_button.props.halign = Gtk.Align.CENTER
        ext_label = Gtk.Label()
        ext_label.set_markup("GNOME Extensions")
        ext_label.props.halign = Gtk.Align.START

        # GDM Settings
        gdm_settings_button = Gtk.Button.new_with_label("Open")
        gdm_settings_button.connect("clicked", self.on_gdm_settings_activated)
        gdm_settings_button.props.valign = Gtk.Align.CENTER
        gdm_settings_button.props.halign = Gtk.Align.CENTER
        gdm_settings_label = Gtk.Label()
        gdm_settings_label.set_markup("GDM Settings")
        gdm_settings_label.props.halign = Gtk.Align.START

        # Accent Colors
        accent_colors_button = Gtk.Button.new_with_label("Open")
        accent_colors_button.connect("clicked", self.on_accent_colors_activated)
        accent_colors_button.props.valign = Gtk.Align.CENTER
        accent_colors_button.props.halign = Gtk.Align.CENTER
        accent_colors_label = Gtk.Label()
        accent_colors_label.set_markup("Accent Colors")
        accent_colors_label.props.halign = Gtk.Align.START

        # Firefox Theme
        firefox_theme_button = Gtk.Button.new_with_label("Open")
        firefox_theme_button.connect("clicked", self.on_firefox_theme_activated)
        firefox_theme_button.props.valign = Gtk.Align.CENTER
        firefox_theme_button.props.halign = Gtk.Align.CENTER
        firefox_theme_label = Gtk.Label()
        firefox_theme_label.set_markup("Firefox Theme")
        firefox_theme_label.props.halign = Gtk.Align.START

        # Theme tab layout
        theme_grid.attach(tweaks_button, 3, 0, 1, 1)
        theme_grid.attach(tweaks_label, 1, 0, 2, 1)
        theme_grid.attach(ext_button, 3, 1, 1, 1)
        theme_grid.attach(ext_label, 1, 1, 2, 1)
        theme_grid.attach(accent_colors_button, 3, 2, 1, 1)
        theme_grid.attach(accent_colors_label, 1, 2, 2, 1)
        theme_grid.attach(gdm_settings_button, 3, 3, 1, 1)
        theme_grid.attach(gdm_settings_label, 1, 3, 2, 1)
        theme_grid.attach(tray_switch, 6, 0, 1, 1)
        theme_grid.attach(tray_label, 4, 0, 2, 1)
        theme_grid.attach(desk_switch, 6, 1, 1, 1)
        theme_grid.attach(desk_label, 4, 1, 2, 1)
        theme_grid.attach(firefox_theme_button, 6, 2, 1, 1)
        theme_grid.attach(firefox_theme_label, 4, 2, 2, 1)


    def set_preview_colors(self, newcolor: str):
        """ load preview images """
        # TODO use property data_dir
        res_directory = Path(__file__).parent / "../../data"  # only if we use git, path exists
        if not res_directory.resolve().exists():
            res_directory = "/usr/share/gls"
        try:
            for key, img in self.previews.items():
                # Normal preview
                shutil.copyfile(
                    f"{res_directory}/pictures/{key}preview.svg", f"{temp_dir}/{key}preview.svg")
                replace_in_file(
                    f"{temp_dir}/{key}preview.svg", "#16a085", self.default_color)
                img.set_from_file(f"{temp_dir}/{key}preview.svg")
                # Selected preview
                shutil.copyfile(
                    f"{res_directory}/pictures/{key}preview.svg", f"{temp_dir}/{key}preview_selected.svg")
                replace_in_file(f"{temp_dir}/{key}preview_selected.svg", "#16a085", newcolor)
        except FileNotFoundError:
            return False

    def create_layout_btn(self, layout, the_grid):
        """ Create desktop element in grid """
        btn = Gtk.RadioButton.new_with_label_from_widget(self.btn_layout_first, layout["label"])
        if not self.btn_layout_first:
            self.btn_layout_first = btn
        btn.connect("toggled", self.on_layout_toggled, layout["id"])
        the_grid.attach(btn, layout["x"], layout["y"], 1, 1)

        preview_img = Gtk.Image()
        self.previews[layout["id"]] = preview_img
        # preview loaded by set_preview_colors()
        btn.image = preview_img  # link img to checkbox for easy find
        # reduce opacity of de-selected previews
        if btn.get_active():
            preview_img.set_opacity(Opacity.TOP)
        else:
            preview_img.set_opacity(Opacity.MIDDLE)
        # make preview images clickable
        event_img = Gtk.EventBox()
        event_img.connect("button-release-event", self.on_click_img)  # click in box
        event_img.connect("enter-notify-event", self.on_over_img, True)  # mouse over box
        event_img.connect("leave-notify-event", self.on_over_img, False)  # mouse out box
        event_img.add(preview_img)  # add img in box
        event_img.btn = btn  # link btn to box for easy find
        the_grid.attach(event_img, layout["x"], layout["y"] + 1, 1, 1)  # add box and not img in grid

    @property
    def current_color(self):
        return self._current_color

    @current_color.setter
    def current_color(self, value):
        """ assign new curent color :
                change preview images
                change btn theme color
                not change file.css

            if not value then read value in file.css
        """
        if not value:
            value = "#16a085"
            try:
                with open(css_file, encoding="utf-8") as fread:
                    file = fread.read()
                    value = re.search("^@define-color.*theme_selected_bg_color.*#(.*);", file)
                    value = f"#{value.group(1)}"
            except FileNotFoundError:
                value = self.highlight_color
            except AttributeError:
                value = self.highlight_color
        self._current_color = value
        self.set_preview_colors(self._current_color)
        # ??? and (re-)change btn theme color
        if self.color_button:
            color = Gdk.RGBA()
            color.parse(self._current_color)
            self.color_button.set_rgba(color)

    ###
    # event functions
    ###

    def on_layout_toggled(self, button, name):
        """ change default layout """
        state = Opacity.LOW
        if button.get_active():
            state = Opacity.TOP
            self.layout = name
            print("active layout:", self.layout)
            button.image.set_from_file(f"{temp_dir}/{self.layout}preview_selected.svg")
        else:
            button.image.set_from_file(f"{temp_dir}/{self.layout}preview.svg")
        button.image.set_opacity(state)  # change img opacity from state

    def on_over_img(self, box, event, is_over_image):
        """ on mouse over / out : change opacity """
        if box.btn.get_active():
            return
        if is_over_image:
            box.btn.image.set_opacity(Opacity.MIDDLE)
        else:
            box.btn.image.set_opacity(Opacity.LOW)

    def on_color_chosen(self, user_data):
        """ after chose a theme color """
        col = self.color_button.get_rgba().to_string()[4:-1:]
        # convert color to hexadecimal
        col = col.split(",")
        col = (int(x) for x in col)
        col = "#%02x%02x%02x" % tuple(col)
        set_highlight_color(col)  # E0602: Undefined variable 'set_highlight_color'
        self.current_color = col

    def on_desk_activated(self, switch, gparam):
        if switch.get_active():
            state = "on"
            subprocess.run("gnome-extensions enable gtk4-ding@smedius.gitlab.com", shell=True, check=False)
        else:
            state = "off"
            subprocess.run("gnome-extensions disable gtk4-ding@smedius.gitlab.com", shell=True, check=False)
        print("Desktop icons was turned", state)

    def on_tray_activated(self, switch, gparam):
        if switch.get_active():
            state = "on"
            subprocess.run(
                "gnome-extensions enable appindicatorsupport@rgcjonas.gmail.com", shell=True, check=False)
        else:
            state = "off"
            subprocess.run(
                "gnome-extensions disable appindicatorsupport@rgcjonas.gmail.com", shell=True, check=False)
        print("System tray was turned", state)

    # ------------- branding -------------------------------------------
    def on_branding_activated(self, switch, gparam):
        self.change_branding(switch)

    def change_branding(self, switch):
        # https://python-gtk-3-tutorial.readthedocs.io/en/latest/basics.html#main-loop-and-signals
        # disconnect the signal
        switch.disconnect(self.branding_handler_id)

        # print entry state aka the new state
        print(f"(ENTRY) switch.get_active() is {switch.get_active()} - new state")
        print(f"(ENTRY) switch.get_state() is {switch.get_state()} - new state")

        # switch branding
        if switch.get_active():
            result, _ = do_branding(False)
            if result:
                self.branding_active = True
        else:
            result, _ = do_branding(True)
            if result:
                self.branding_active = False
        if not result:
            self.branding_active = get_asset_state()
            switch.set_active(self.branding_active)

        # reconnect signal
        self.branding_handler_id = switch.connect("notify::active", self.on_branding_activated)

        # print the exit state
        print(f"(EXIT) switch.get_active() is {switch.get_active()} - final state")
        print(f"(EXIT) switch.get_state() is {switch.get_state()} - final state")
    # ------------- end branding ---------------------------------------

    def on_gnometweaks_activated(self, button):
        subprocess.Popen("gnome-tweaks")

    def on_gnomext_activated(self, button):
        subprocess.Popen("gnome-extensions-app")

    def on_accent_colors_activated(self, button):
        subprocess.Popen("kgx -T 'Choose Your Accent Color' -e 'accent-color-change'", shell=True,)

    def on_gdm_settings_activated(self, button):
        subprocess.Popen("gdm-settings", shell=True,)

    def on_firefox_theme_activated(self, button):
        subprocess.Popen("addwater", shell=True,)

    def on_click_img(self, box, event):
        """Make images clickable
            change checkbox state and call on_layout_toggled()"""
        box.btn.set_active(True)

    def dialog_error(self, title: str, message: str):
        """display error modal dialog"""
        dialog = Gtk.MessageDialog(
            parent=self.window,
            flags=Gtk.DialogFlags.MODAL | Gtk.DialogFlags.DESTROY_WITH_PARENT,
            type=Gtk.MessageType.ERROR,
            buttons=Gtk.ButtonsType.CLOSE,
            message_format=title
        )
        dialog.format_secondary_text(message)
        dialog.run()
        dialog.destroy()

    def on_reload_clicked(self, button):
        reload_gnome_shell(standalone_mode=False)

    def on_layoutapply_clicked(self, button):
        """ apply default layout to user """

        switch_layout = f"apply_{self.layout}(standalone_mode=False)"
        eval(switch_layout)
        shell('gsettings set org.gnome.shell disable-user-extensions false')

class Command(Gtk.Box):

    @click.group()
    def cli():
        pass

    cli.add_command(apply_traditional)
    cli.add_command(apply_manjaro)
    cli.add_command(apply_gnome)
    cli.add_command(apply_tiling)
