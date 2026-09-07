# Layout Schema (Synthesis Baseline)

This document defines one common model for comparing and authoring GNOME layouts during synthesis work.

## Logical schema

Each layout should be described with these logical blocks:

- `required_extensions`: extensions that must end up enabled
- `conflicting_extensions`: extensions that must end up disabled
- `settings`: core layout settings (`dock/panel/menu/button-layout`) applied via `dconf`/`gsettings`
- `health_checks`: checks that confirm the resulting layout state

## Mapping to `.de` (engine-supported fields)

The runtime engine does not have dedicated keys called `required_extensions` or `conflicting_extensions`.
Use this mapping:

- `required_extensions` -> `steps` with:
  - `type: extension.install` (or `extension.install_bundled`)
  - optionally `type: extension.enable`
- `conflicting_extensions` -> `steps` with:
  - `type: extension.disable`
- `settings` -> `steps` with:
  - `type: dconf` (`action: load`) for grouped extension settings
  - `type: gsetting` or `type: bash` for focused keys
- `health_checks` -> `desktops.<de>.health_checks`

## Minimal authoring template

```yaml
id: example_layout
label: Example Layout
preview: gnome
description: Example
spec_version: 1
provider: synthesis
variant: example
capabilities_required:
  - dconf
  - gsettings
  - extensions.install
  - extensions.disable
  - gnome.shell_reload
capabilities_optional:
  - preflight_checks
  - health_checks
degraded_behavior: Apply core layout even if optional pieces fail.
rollback_policy: snapshot
desktops:
  gnome:
    preflight_checks:
      - type: bash
        command: "command -v dconf >/dev/null"
    health_checks:
      - type: bash
        command: "gsettings get org.gnome.shell enabled-extensions | grep -q 'dash-to-dock@micxgx.gmail.com'"
    steps:
      - type: extension.install
        id: dash-to-dock@micxgx.gmail.com
      - type: dconf
        path: /org/gnome/
        action: load
        data: |
          [shell/extensions/dash-to-dock]
          dock-position='BOTTOM'
      - type: gnome.shell_reload
      - type: extension.disable
        id: dash-to-panel@jderose9.github.com
```

