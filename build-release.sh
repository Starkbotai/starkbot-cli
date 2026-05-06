#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────────────────────
# build-release.sh — Build StarkBot release binaries
#
# Usage:
#   ./build-release.sh                  # build for current platform
#   ./build-release.sh --target linux   # Linux x86_64 + aarch64 (via cross)
#   ./build-release.sh --target all     # all supported targets
#   ./build-release.sh --version 0.2.0  # override version
#
# Note: macOS and Windows builds require GitHub Actions CI.
#       See .github/workflows/release.yml
# ──────────────────────────────────────────────────────────────

VERSION=""
TARGET="native"
RELEASE_DIR="releases"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)  TARGET="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        --help|-h)
            sed -n '3,14p' "$0"
            exit 0 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Auto-detect version from Cargo.toml if not specified
if [[ -z "$VERSION" ]]; then
    VERSION=$(grep '^version' crates/starkbot-tauri/Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
fi

echo "==> Building StarkBot v${VERSION}"

# Map friendly names to Rust target triples
declare -A TARGET_MAP=(
    [linux-x86_64]="x86_64-unknown-linux-gnu"
    [linux-aarch64]="aarch64-unknown-linux-gnu"
    [macos-x86_64]="x86_64-apple-darwin"
    [macos-aarch64]="aarch64-apple-darwin"
    [windows-x86_64]="x86_64-pc-windows-msvc"
)

# Determine which targets to build
TARGETS=()
case "$TARGET" in
    native)
        TARGETS=("native")
        ;;
    linux)
        TARGETS=("linux-x86_64" "linux-aarch64")
        ;;
    macos)
        TARGETS=("macos-x86_64" "macos-aarch64")
        ;;
    windows)
        TARGETS=("windows-x86_64")
        ;;
    all)
        # Detect what we can actually build
        TARGETS=("linux-x86_64" "linux-aarch64")
        OS="$(uname -s)"
        if [[ "$OS" == "Darwin" ]]; then
            TARGETS+=("macos-x86_64" "macos-aarch64")
        else
            echo "==> Skipping macOS targets (requires macOS host or CI)"
        fi
        echo "==> Skipping Windows target (use GitHub Actions CI)"
        ;;
    *)
        echo "Unknown target: $TARGET (use linux, macos, windows, all, or native)"
        exit 1
        ;;
esac

# Check for cross when doing cross-compilation (non-native Linux targets)
USE_CROSS=false
if [[ "$TARGET" != "native" ]]; then
    if command -v cross &>/dev/null; then
        USE_CROSS=true
        echo "==> Using 'cross' for cross-compilation"
    else
        echo "WARNING: 'cross' not found. Install with: cargo install cross"
        echo "         Falling back to cargo (may fail for non-native targets)"
    fi
fi

mkdir -p "$RELEASE_DIR"

BUILT=()

build_target() {
    local label="$1"
    local bin_suffix=""
    local archive_ext="tar.gz"

    if [[ "$label" == *"windows"* ]]; then
        bin_suffix=".exe"
        archive_ext="zip"
    fi

    local archive_name="starkbot-v${VERSION}-${label}"
    local staging_dir="${RELEASE_DIR}/${archive_name}"

    echo ""
    echo "──────────────────────────────────────────"
    echo "Building: ${label}"
    echo "──────────────────────────────────────────"

    # Build
    if [[ "$label" == "native" ]]; then
        cargo build --release -p starkbot-tauri
        local bin_dir="target/release"
    else
        local triple="${TARGET_MAP[$label]}"
        if $USE_CROSS; then
            cross build --release -p starkbot-tauri --target "$triple"
        else
            cargo build --release -p starkbot-tauri --target "$triple"
        fi
        local bin_dir="target/${triple}/release"
    fi

    # Package
    rm -rf "$staging_dir"
    mkdir -p "$staging_dir"

    cp "${bin_dir}/starkbot-gui${bin_suffix}" "$staging_dir/"
    cp README.md "$staging_dir/" 2>/dev/null || true
    cp LICENSE* "$staging_dir/" 2>/dev/null || true

    # Include default personas and skills
    if [[ -d "personas" ]]; then
        cp -r personas "$staging_dir/"
    fi
    if [[ -d "skills" ]]; then
        cp -r skills "$staging_dir/"
    fi

    # Create archive
    cd "$RELEASE_DIR"
    if [[ "$archive_ext" == "zip" ]]; then
        zip -qr "${archive_name}.zip" "$archive_name"
        echo "==> Created: ${RELEASE_DIR}/${archive_name}.zip"
    else
        tar czf "${archive_name}.tar.gz" "$archive_name"
        echo "==> Created: ${RELEASE_DIR}/${archive_name}.tar.gz"
    fi
    cd ..

    # Clean up staging dir
    rm -rf "$staging_dir"
    BUILT+=("$archive_name")
}

# Build each target
for t in "${TARGETS[@]}"; do
    build_target "$t"
done

# Generate checksums
echo ""
echo "==> Generating checksums"
cd "$RELEASE_DIR"
sha256sum starkbot-v${VERSION}-*.{tar.gz,zip} 2>/dev/null > "checksums-v${VERSION}.sha256" || true
cat "checksums-v${VERSION}.sha256"
cd ..

# Summary
echo ""
echo "==> Release artifacts in ${RELEASE_DIR}/:"
ls -lh "$RELEASE_DIR"/starkbot-v${VERSION}-* "$RELEASE_DIR"/checksums-* 2>/dev/null
echo ""
echo "==> Done! Next steps:"
echo "    1. Update RELEASES.md with the new version"
echo "    2. git tag v${VERSION}"
echo "    3. git push --tags"
echo "    4. Upload artifacts to GitHub release"
