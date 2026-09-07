# Refract

Your desktop, any shape

# Desktop Experience Switcher

Refract is a desktop utility for switching between preconfigured GNOME and KDE layouts.

The project focuses on practical desktop presets such as

* **GNOME (Vanilla)** (`gnome`): A clean and classic GNOME environment.
* **Manjaro** (`manjaro`): A layout inspired by the Manjaro distribution.
* **Material Shell** (`material_shell`): A modern interface based on Material Design concepts.
* **Tiling (Forge)** (`tiling`): A tiling layout using the Forge extension.
* **Traditional** (`traditional`): A classic layout with a bottom panel and a window list.
* **Unity** (`unity`): A layout inspired by the former Ubuntu Unity interface with a side dock.

## Features

- Apply predefined GNOME layouts from declarative `.de` preset files
- Manage extension-based layouts together with shell settings and `dconf` changes
- Create automatic snapshots before and after changes
- Restore previous desktop state from snapshot history
- Delete old snapshots directly from the history view
- Run preflight checks, health checks, and rollback on failed apply
- Use bundled Unity extensions shipped with the app instead of downloading them at apply time
- Export the current GNOME layout and use it later as a custom preset
- Generate developer snapshots and compatibility reports for extension troubleshooting

## Project Status

The project is in active development and already usable as an advanced prototype for GNOME layout switching.

Current state:

- The GNOME layout engine is implemented and supports rollback-oriented apply flow
- Multiple presets are available out of the box, including a bundled Unity preset
- History, snapshots, and developer tooling are integrated into the GUI
- Some layouts still depend on online extension sources, while Unity already uses local bundled extensions
- The app should be treated as experimental software rather than a finished end-user product

## Installation

### Requirements

- Linux with GNOME
- Rust toolchain with Cargo
- `gtk4` development libraries installed on the system

### Run in development

#### Build a portable dist folder

```bash
cd desktop-experience-switcher
./dev/build_prod.sh
```

This creates a portable `dist` folder.

Run the packaged app with:

```bash
./dist/desktop-experience-switcher/run.sh
```

## Developer Utilities

Useful helper scripts are available in the `dev` directory:

- `build_prod.sh` builds the distributable folder
- `build_unity_extensions.sh` builds the vendored Ubuntu and Unity extension bundle
- `check_gnome_extensions.py` checks extension metadata, versions, and GNOME compatibility
- `check_layout_extensions.sh` checks all unique extension UUIDs referenced by layout files

## License

GNU GPLv3
See LICENSE file
