This directory holds third-party extension sources that we vendor for Unity-shell-related packaging work.

`ubuntu-base/` is the imported Ubuntu `gnome-shell-ubuntu-extensions` source tree that we can build locally as a starting point for a future Unity-oriented package.

Use `dev/build_unity_extensions.sh` to fetch Meson subprojects and build the staged install under `target/unity-shell-extensions/`.
