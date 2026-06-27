#!/bin/bash
set -e

# Usage: ./build-macos-appstore.sh <binary_path>
BINARY_PATH="$1"

if [ -z "$BINARY_PATH" ]; then
  echo "Usage: $0 <path_to_binary>"
  exit 1
fi

APP_NAME="udp-packet-studio"
APP_BUNDLE="${APP_NAME}.app"
CONTENTS_DIR="${APP_BUNDLE}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
PKG_NAME="${APP_NAME}.pkg"

echo "Creating App Store Bundle structure at ${APP_BUNDLE}..."
rm -rf "${APP_BUNDLE}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

echo "Copying binary..."
cp "${BINARY_PATH}" "${MACOS_DIR}/${APP_NAME}"
chmod +x "${MACOS_DIR}/${APP_NAME}"

# Handle Icon file if it exists
HAS_ICON=false
# Search for icon.icns in project root or scripts directory
if [ -f "icon.icns" ]; then
  echo "Found icon.icns in project root. Copying to Resources..."
  cp "icon.icns" "${RESOURCES_DIR}/"
  HAS_ICON=true
elif [ -f "scripts/icon.icns" ]; then
  echo "Found icon.icns in scripts directory. Copying to Resources..."
  cp "scripts/icon.icns" "${RESOURCES_DIR}/"
  HAS_ICON=true
else
  echo "Warning: icon.icns not found. Proceeding without App Icon (App Store submission will require one!)."
fi

# Extract version from Cargo.toml
VERSION=$(grep -m 1 '^version =' Cargo.toml | cut -d '"' -f 2)
if [ -z "$VERSION" ]; then
  VERSION="0.1.0"
fi

echo "Generating Info.plist (version: ${VERSION})..."
cat <<EOF > "${CONTENTS_DIR}/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>jp.daradara.${APP_NAME}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>UDP Packet Studio</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>ITSAppUsesNonExemptEncryption</key>
    <false/>
$(if [ "$HAS_ICON" = true ]; then
  echo "    <key>CFBundleIconFile</key>"
  echo "    <string>icon.icns</string>"
fi)
</dict>
</plist>
EOF

# Find Apple Distribution certificate in keychain if not set
if [ -z "$CODESIGN_IDENTITY" ]; then
  echo "CODESIGN_IDENTITY not set. Searching keychain for Apple Distribution certificate..."
  CODESIGN_IDENTITY=$(security find-identity -v -p codesigning | grep "Apple Distribution" | head -n 1 | awk -F '"' '{print $2}')
fi

# Find Mac Installer Distribution certificate in keychain if not set
if [ -z "$INSTALLER_IDENTITY" ]; then
  echo "INSTALLER_IDENTITY not set. Searching keychain for Mac Installer Distribution certificate..."
  INSTALLER_IDENTITY=$(security find-identity -v | grep "3rd Party Mac Developer Installer\|Mac Installer Distribution" | head -n 1 | awk -F '"' '{print $2}')
fi

# 1. Embed provisioning profile
if [ -z "$PROVISIONING_PROFILE" ]; then
  # Search for any .provisionprofile file in current directory or scripts/
  PROVISIONING_PROFILE=$(find . -maxdepth 2 -name "*.provisionprofile" | head -n 1)
fi

if [ -n "$PROVISIONING_PROFILE" ]; then
  echo "Embedding provisioning profile: ${PROVISIONING_PROFILE}"
  cp "${PROVISIONING_PROFILE}" "${CONTENTS_DIR}/embedded.provisionprofile"
else
  echo "Warning: No provisioning profile found. Set PROVISIONING_PROFILE env var or place a .provisionprofile file here."
  echo "You will not be able to submit to TestFlight or the App Store without it."
fi

# 1b. Remove extended attributes (like com.apple.quarantine) from all files in the app bundle
echo "Removing extended attributes (quarantine) from bundle..."
xattr -cr "${APP_BUNDLE}"

# 2. Sign the app bundle (after all assets and profiles are inside)
if [ -n "$CODESIGN_IDENTITY" ]; then
  echo "Signing app binary and bundle with identity: ${CODESIGN_IDENTITY}"
  
  # Sign the executable inside the bundle (must use Sandboxed entitlements, no hardened runtime option is needed for App Store)
  codesign --force --entitlements scripts/entitlements.appstore.plist --sign "${CODESIGN_IDENTITY}" --timestamp "${MACOS_DIR}/${APP_NAME}"
  
  # Sign the bundle itself
  codesign --force --entitlements scripts/entitlements.appstore.plist --sign "${CODESIGN_IDENTITY}" --timestamp "${APP_BUNDLE}"
  
  echo "Verification of App Bundle signature:"
  codesign --verify --verbose --deep "${APP_BUNDLE}"
else
  echo "Warning: No Apple Distribution codesign identity found. Skipping App Bundle codesigning."
  echo "You will not be able to submit to the App Store without codesigning."
fi

# 2. Build the Installer Package (.pkg)
rm -f "${PKG_NAME}"
if [ -n "$INSTALLER_IDENTITY" ]; then
  echo "Building and signing Installer Package (.pkg) with identity: ${INSTALLER_IDENTITY}"
  productbuild --component "${APP_BUNDLE}" /Applications --sign "${INSTALLER_IDENTITY}" "${PKG_NAME}"
  
  echo "Verification of Installer Package (.pkg):"
  pkgutil --check-signature "${PKG_NAME}"
  
  echo "--------------------------------------------------------"
  echo "Successfully built and signed App Store package: ${PKG_NAME}"
  echo "You can upload this package using the Transporter app"
  echo "or xcrun altool."
  echo "--------------------------------------------------------"
else
  echo "Warning: No Mac Installer Distribution identity found. Building UNSIGNED package..."
  productbuild --component "${APP_BUNDLE}" /Applications "${PKG_NAME}"
  
  echo "--------------------------------------------------------"
  echo "Built UNSIGNED package: ${PKG_NAME}"
  echo "Note: You must sign this package before uploading to App Store Connect."
  echo "--------------------------------------------------------"
fi
