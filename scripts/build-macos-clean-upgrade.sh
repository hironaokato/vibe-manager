#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
VERSION=$(node -p "require('$PROJECT_ROOT/package.json').version")
TARGET_ROOT="$PROJECT_ROOT/src-tauri/target/universal-apple-darwin/release"
APP_PATH="$TARGET_ROOT/bundle/macos/Vibe Manager.app"
BUNDLE_ROOT="$TARGET_ROOT/bundle"
PKG_DIR="$BUNDLE_ROOT/pkg"
FINAL_PKG="$PKG_DIR/Vibe Manager_${VERSION}_universal.pkg"
COMPONENT_PKG="$PKG_DIR/Vibe Manager_${VERSION}_component.pkg"
PKG_SCRIPTS="$PROJECT_ROOT/src-tauri/macos/pkg-scripts"

cd "$PROJECT_ROOT"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npx tauri build --bundles app --target universal-apple-darwin

if [ ! -d "$APP_PATH" ]; then
  echo "Built application bundle was not found: $APP_PATH" >&2
  exit 1
fi

mkdir -p "$PKG_DIR"
rm -f "$FINAL_PKG"
PACKAGE_ROOT=$(mktemp -d "$TARGET_ROOT/vibe-manager-pkg-root.XXXXXX")
cleanup() {
  case "$PACKAGE_ROOT" in
    "$TARGET_ROOT"/vibe-manager-pkg-root.*) rm -rf "$PACKAGE_ROOT" ;;
    *) echo "Refusing to clean unexpected package root: $PACKAGE_ROOT" >&2 ;;
  esac
  rm -f "$COMPONENT_PKG"
}
trap cleanup EXIT INT TERM

mkdir -p "$PACKAGE_ROOT/Applications"
ditto "$APP_PATH" "$PACKAGE_ROOT/Applications/Vibe Manager.app"
chmod 755 "$PKG_SCRIPTS/preinstall"

pkgbuild \
  --root "$PACKAGE_ROOT" \
  --scripts "$PKG_SCRIPTS" \
  --identifier "app.vibemanager.desktop" \
  --version "$VERSION" \
  --install-location "/" \
  "$COMPONENT_PKG"

productbuild --package "$COMPONENT_PKG" "$FINAL_PKG"

# Keep the PKG as the only macOS installer artifact. A PKG can run the clean
# replacement script; a drag-and-drop DMG cannot enforce that policy.
find "$BUNDLE_ROOT" -type f \( -name "*.dmg" -o -name "*.pkg" \) ! -path "$FINAL_PKG" -delete

echo "Clean-upgrade installer: $FINAL_PKG"
shasum -a 256 "$FINAL_PKG"
