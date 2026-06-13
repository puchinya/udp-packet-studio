#!/bin/bash
set -e

# Usage: ./build-macos.sh <binary_path>
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
DMG_NAME="${APP_NAME}.dmg"

echo "Creating App Bundle structure at ${APP_BUNDLE}..."
rm -rf "${APP_BUNDLE}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

echo "Copying binary..."
cp "${BINARY_PATH}" "${MACOS_DIR}/${APP_NAME}"
chmod +x "${MACOS_DIR}/${APP_NAME}"

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
    <string>com.puchinya.${APP_NAME}</string>
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
    <string>10.12</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Codesign identity
if [ -z "$CODESIGN_IDENTITY" ]; then
  echo "CODESIGN_IDENTITY not set. Searching keychain for Developer ID Application certificate..."
  # List codesigning identities, grep for Developer ID, grab the first one between quotes
  CODESIGN_IDENTITY=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -n 1 | awk -F '"' '{print $2}')
fi

if [ -n "$CODESIGN_IDENTITY" ]; then
  echo "Signing app binary and bundle with identity: ${CODESIGN_IDENTITY}"
  
  # Sign the executable inside the bundle
  codesign --force --options runtime --entitlements scripts/entitlements.plist --sign "${CODESIGN_IDENTITY}" --timestamp "${MACOS_DIR}/${APP_NAME}"
  
  # Sign the bundle itself
  codesign --force --options runtime --entitlements scripts/entitlements.plist --sign "${CODESIGN_IDENTITY}" --timestamp "${APP_BUNDLE}"
  
  echo "Verification of App Bundle signature:"
  codesign --verify --verbose --deep "${APP_BUNDLE}"
else
  echo "Warning: No codesign identity found. Skipping App Bundle codesigning."
fi

# Create a temporary staging directory for DMG
STAGING_DIR="dmg_staging"
echo "Creating DMG structure..."
rm -rf "${STAGING_DIR}" "${DMG_NAME}"
mkdir -p "${STAGING_DIR}"

# Copy the signed .app to staging
cp -R "${APP_BUNDLE}" "${STAGING_DIR}/"

# Create a symlink to Applications directory for drag-and-drop installation
ln -s /Applications "${STAGING_DIR}/Applications"

# Generate DMG using hdiutil
echo "Building DMG image..."
hdiutil create -volname "UDP Packet Studio" -srcfolder "${STAGING_DIR}" -ov -format UDZO "${DMG_NAME}"
rm -rf "${STAGING_DIR}"

# Codesign the DMG itself (required for notarization of DMG files)
if [ -n "$CODESIGN_IDENTITY" ]; then
  echo "Signing DMG with identity: ${CODESIGN_IDENTITY}"
  codesign --force --sign "${CODESIGN_IDENTITY}" --timestamp "${DMG_NAME}"
  
  echo "Verification of DMG signature:"
  codesign --verify --verbose "${DMG_NAME}"
else
  echo "Warning: No codesign identity found. Skipping DMG codesigning."
fi

# Notarization
if [ -n "$APPLE_ID" ] && [ -n "$APPLE_APP_SPECIFIC_PASSWORD" ] && [ -n "$APPLE_TEAM_ID" ]; then
  if [ -z "$CODESIGN_IDENTITY" ]; then
    echo "Error: Cannot notarize an unsigned DMG. Please provide a codesign certificate."
    exit 1
  fi
  
  echo "Submitting DMG to Apple Notary Service..."
  xcrun notarytool submit "${DMG_NAME}" \
    --apple-id "${APPLE_ID}" \
    --password "${APPLE_APP_SPECIFIC_PASSWORD}" \
    --team-id "${APPLE_TEAM_ID}" \
    --wait
  
  echo "Stapling notarization ticket to DMG..."
  xcrun stapler staple "${DMG_NAME}"
  
  echo "Verification of stapling..."
  spctl --assess --verbose=4 --type open --context context:primary-signature "${DMG_NAME}" || echo "spctl check skipped or failed"
  
  echo "Successfully signed, notarized, and packaged DMG!"
else
  echo "Warning: Apple Notary credentials not fully set. Skipping notarization."
  echo "Created package without notarization."
fi
