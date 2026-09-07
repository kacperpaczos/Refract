#!/usr/bin/env bash

set -e

# Przejście do głównego katalogu workspace'a (jedno w górę z katalogu /dev/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

DIST_DIR="$ROOT_DIR/dist/desktop-experience-switcher"

echo "=================================================="
echo " Desktop Experience Switcher — Build Produkcyjny"
echo "=================================================="
echo ""
echo "[1/3] Kompilacja Rust (profil release)..."
cargo build --release --bin ui

echo ""
echo "[2/3] Tworzenie folderu dystrybucyjnego: dist/"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Binarka
cp target/release/ui "$DIST_DIR/desktop-experience-switcher"

# Dane aplikacji: layouty, obrazki
cp -r data/ "$DIST_DIR/data/"

# Ikona (jeśli istnieje)
if [ -d "assets" ]; then
    cp -r assets/ "$DIST_DIR/assets/"
fi

# Skrypt uruchamiający (wrapper z poprawną ścieżką do data/)
cat > "$DIST_DIR/run.sh" << 'EOF'
#!/usr/bin/env bash
# Uruchamia aplikację z poprawną ścieżką do danych
APP_DIR="$(cd "$(dirname "$0")" && pwd)"
export DESKTOP_EXPERIENCE_DATA_DIR="$APP_DIR/data"
exec "$APP_DIR/desktop-experience-switcher" "$@"
EOF
chmod +x "$DIST_DIR/run.sh"

echo ""
echo "[3/3] Gotowe! Folder dystrybucyjny:"
echo ""
echo "  📁 dist/desktop-experience-switcher/"
echo "     ├── desktop-experience-switcher  (binarka)"
echo "     ├── run.sh                       (skrypt uruchamiający)"
echo "     └── data/"
echo "         ├── layouts/*.de             (definicje środowisk)"
echo "         └── pictures/*.svg           (podglądy layoutów)"
echo ""
echo "  Przenieś folder 'desktop-experience-switcher/' gdziekolwiek i uruchom:"
echo "  ./run.sh"
echo "=================================================="
