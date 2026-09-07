#!/usr/bin/env bash

# Skrypt deweloperski z pełną diagnostyką
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

echo "============================================"
echo " Desktop Experience Switcher — DEV MODE"
echo "============================================"
echo ""

# ── [1/4] Sprawdzenie struktury plików ───────────
echo "[1/4] Weryfikacja struktury plików:"
ERRORS=0

check() {
    if [ -e "$1" ]; then
        echo "  ✅ $1"
    else
        echo "  ❌ BRAK: $1"
        ERRORS=$((ERRORS + 1))
    fi
}

check "data/layouts"
check "data/pictures"
LAYOUT_COUNT=$(find data/layouts -name "*.de" 2>/dev/null | wc -l)
echo "  📄 Znaleziono $LAYOUT_COUNT plików layoutów (.de):"
find data/layouts -name "*.de" 2>/dev/null | sort | while read -r f; do
    echo "     → $f"
done

if [ "$ERRORS" -gt 0 ]; then
    echo ""
    echo "⛔ Wykryto $ERRORS błędów struktury! Sprawdź powyższe ścieżki."
    exit 1
fi

# ── [2/4] Sprawdzenie bibliotek GTK4 ─────────────
echo ""
echo "[2/4] Sprawdzenie bibliotek systemowych GTK4:"
for lib in libgtk-4.so.1 libglib-2.0.so.0 libgio-2.0.so.0 libgdk-4.so.1; do
    if ldconfig -p 2>/dev/null | grep -q "$lib"; then
        echo "  ✅ $lib"
    else
        echo "  ⚠️  $lib — nie znaleziono (może być niezbędna)"
    fi
done

# ── [3/4] Kompilacja w trybie dev (debugowanie) ──
echo ""
echo "[3/4] Kompilacja (debug, wszystkie ostrzeżenia)..."
cargo build --bin ui 2>&1
if [ $? -ne 0 ]; then
    echo "⛔ Kompilacja nie powiodła się!"
    exit 1
fi

# ── [4/4] Uruchomienie ───────────────────────────
echo ""
echo "[4/4] Uruchamianie aplikacji (RUST_LOG=debug)..."
echo "      Logi -> konsola + logs/log.txt"
echo "--------------------------------------------"
export RUST_LOG=debug
export RUST_BACKTRACE=1
export DESKTOP_EXPERIENCE_DATA_DIR="$ROOT_DIR/data"
export G_MESSAGES_DEBUG=all
exec ./target/debug/ui
