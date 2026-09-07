# Pkt 3: Porównanie layoutów (GLS vs Linexin) i realny wkład GLS

## Zakres porównania

Źródła:
- `references/gnome-layout-switcher-master/bin/layoutswitcherlib/layoutsbox.py`
- `references/linexin-desktop-presets/src/etc/skel/.local/share/linexin/linexin-desktop/default.sh`
- `references/linexin-desktop-presets/src/etc/skel/.local/share/linexin/linexin-desktop/windowish.sh`
- `references/linexin-desktop-presets/src/etc/skel/.local/share/linexin/linexin-desktop/ubunexin.sh`
- `references/linexin-desktop-presets/src/etc/skel/.local/share/linexin/linexin-desktop/gnome.sh`
- nowe layouty syntezowe `.de`:
  - `Refract/data/layouts/gnome_vanilla.de`
  - `Refract/data/layouts/dock_bottom.de`
  - `Refract/data/layouts/panel_traditional.de`
  - `Refract/data/layouts/unity_left_dock.de`
  - `Refract/data/layouts/tiling_forge.de`

Uwaga: pominięto świadomie kategorie „wygląd” i „utility” (np. blur, rounded corners, accent, arch-update, quick-settings-audio-panel, pip-on-top).

## 1) Required extensions: co wnosi GLS ponad Linexin

Linexin (GNOME) dostarcza 3 workflow:
- dock (`default.sh`, `ubunexin.sh`) z `dash-to-dock` + `appindicatorsupport` (+ opcjonalnie `gtk4-ding`)
- panel (`windowish.sh`) z `dash-to-panel` + `arcmenu` + `appindicatorsupport` + `gtk4-ding`
- vanilla (`gnome.sh`) bez rozszerzeń

GLS wnosi ponad to:
- jawne wprowadzenie `gnome-ui-tune@itstime.tech` jako składnika layoutów innych niż vanilla:
  - Traditional (`layoutsbox.py`: `apply_traditional`)
  - Manjaro (`apply_manjaro`)
  - Tiling (`apply_tiling`)
- nowy profil `tiling` jako osobny „first-class” layout:
  - `forge@jmmaranan.com`
  - `space-bar@luchrioh`
  - plus `arcmenu` + `appindicatorsupport` + `gnome-ui-tune`

Mapowanie do nowych `.de`:
- `tiling_forge.de` odzwierciedla dodaną przez GLS rodzinę rozszerzeń tiling.
- `panel_traditional.de` i `dock_bottom.de` zachowują rdzeń Linexin i rozszerzają go o `gnome-ui-tune` tam, gdzie to sensowne (panel).

## 2) Conflicting extensions: co wnosi GLS ponad Linexin

Linexin:
- praktycznie nie ma modelu „conflicting_extensions”
- robi twarde nadpisanie `org.gnome.shell enabled-extensions` (czyli stan globalny), bez deklaratywnej listy konfliktów

GLS:
- wprowadza jawne listy konfliktów per layout i selektywne `gnome-extensions disable`.
- dodatkowo uwzględnia konflikty tilingowe i shellowe, których Linexin nie modeluje:
  - `material-shell@papyelgringo`
  - `forge@jmmaranan.com` / `space-bar@luchrioh` (konflikty między rodzinami layoutów)
  - `pop-shell@system76.com`
  - `workspace-indicator@gnome-shell-extensions.gcampax.github.com`
  - `unite@hardpixel.eu`
  - `no-overview@fthx`
  - `vertical-overview@RensAlthuis.github.com`
  - `places-menu@gnome-shell-extensions.gcampax.github.com`
  - `window-list@gnome-shell-extensions.gcampax.github.com`

Mapowanie do nowych `.de`:
- wszystkie 5 layoutów mają już podejście „disable konfliktów” zamiast samego nadpisywania listy enabled.
- luka względem GLS: `tiling_forge.de` nie wyłącza jeszcze `pop-shell@system76.com` i `workspace-indicator@gnome-shell-extensions.gcampax.github.com` (oba występują jako konflikty w `apply_tiling`).

## 3) Dock / panel / menu / tiling workflow

Dock workflow:
- Linexin: dock przez INI i dconf load (`default.sh`, `ubunexin.sh`), bez jawnej semantyki konfliktów.
- GLS: dock ma własny profil (`manjaro`) + jawne konflikty + konkretny zestaw kluczy (`dock-position BOTTOM`, `extend-height=false`, `dock-fixed=false`).
- `.de`: `dock_bottom.de` odwzorowuje ten model jako deklaratywne kroki (`extension.install`, `dconf load`, `extension.disable`, `health_checks`).

Panel workflow:
- Linexin (`windowish.sh`) dostarcza panel + ArcMenu, ale konfiguracja szczegółowa siedzi w plikach `.ini`.
- GLS (`apply_traditional`) wnosi precyzyjne ustawienie panelu i ArcMenu jako zestaw jawnych kluczy gsettings (bez ukrycia w zewnętrznym INI), np. `panel-element-positions`, `panel-size`, `menu-layout`, `hide-overview-on-startup`.
- `.de`: `panel_traditional.de` realizuje ten sam typ workflow, ale z wariantem Winexin (`menu-layout='Windows'`, inne `panel-element-positions`).

Menu workflow:
- Linexin: ArcMenu tylko w profilu panelowym.
- GLS: ArcMenu także częścią profilu tiling (menu jako launcher do workflow klawiaturowego), z profilem `GnomeOverview`.
- `.de`: `tiling_forge.de` zachowuje ten sam wzorzec (`arcmenu` + `menu-layout='GnomeOverview'`).

Tiling workflow:
- Linexin: brak osobnego workflow tiling.
- GLS: pełny profil tiling (Forge + Space Bar + ArcMenu + konflikty + ustawienia shortcutów).
- `.de`: `tiling_forge.de` to bezpośrednia materializacja największej nowej wartości GLS ponad Linexin.

## 4) Button-layout (różnice krytyczne)

Linexin:
- `default.sh`: `close,minimize,maximize:`
- `windowish.sh` / `ubunexin.sh`: `:minimize,maximize,close`
- `gnome.sh`: `:close`

GLS:
- Traditional / GNOME / Tiling: `:minimize,maximize,close`
- Manjaro: brak jawnej zmiany button-layout (dziedziczy bieżący stan)

Nowe `.de`:
- `gnome_vanilla.de`: `:close` (zgodne z Linexin gnome)
- `dock_bottom.de`: `close,minimize,maximize:` (zgodne z Linexin default)
- `panel_traditional.de`: `:minimize,maximize,close` (zgodne z GLS/Linexin windowish)
- `unity_left_dock.de`: `close,minimize,maximize:` (wariant prawostronny)
- `tiling_forge.de`: `:minimize,maximize,close` (zgodne z GLS tiling)

Wniosek: button-layout nie jest jednolity między źródłami; nowe `.de` poprawnie zachowują rozróżnienie per workflow.

## 5) Krytyczne ustawienia dconf/gsettings

To, co GLS wnosi realnie ponad bazę Linexin (w obszarze funkcjonalnym):
- jawne, per-layout klucze dla ArcMenu i panelu/docka w kodzie, zamiast „czarnej skrzynki” INI,
- jawny reset i ustawienie skrótu Space Bar dla tilingu (`open-menu @as []`),
- jawne listy konfliktów (selektory bezpieczeństwa wdrożenia),
- spójny model aktywacji/dezaktywacji rozszerzeń zależny od layoutu.

To, co Linexin ma, a GLS nie wzmacnia wprost:
- silne profile dconf przez gotowe pliki INI dla konkretnych rozszerzeń,
- pełne ustawienie listy `enabled-extensions` jednym strzałem (szybkie, ale mniej granularne i mniej odporne).

## 6) Konkretna lista „wartości dodanej GLS ponad Linexin” (bez wyglądu i utility)

1. Jawny model konfliktów rozszerzeń per layout (`conflicting_extensions`) zamiast tylko nadpisania `enabled-extensions`.
2. Dedykowany workflow tiling (Forge + Space Bar + ArcMenu + konflikty), którego Linexin GNOME nie miał.
3. Wprowadzenie `gnome-ui-tune` jako elementu funkcjonalnego wspierającego profile nie-vanilla.
4. Deklaratywne sterowanie menu/panelem/dockiem kluczami gsettings na poziomie layoutu (łatwiejsze diffowanie i walidacja).
5. Lepsza separacja rodzin workflow (dock vs panel vs tiling) przez selektywne `enable/disable`, nie tylko przez jedną globalną listę enabled.
6. Lepszy fundament pod health-checki i preflight (bo konflikty i ustawienia są jawne i per-layout).

## 7) Uwagi do bieżących layoutów syntezowych

- `tiling_forge.de`: warto dodać disable dla:
  - `pop-shell@system76.com`
  - `workspace-indicator@gnome-shell-extensions.gcampax.github.com`
  aby domknąć zgodność z logiką konfliktów `apply_tiling` w GLS.
- `panel_traditional.de`: ArcMenu ma wariant `Windows` (Winexin), a nie `Default` jak Traditional z GLS; to świadomy fork funkcjonalny, nie błąd.
