#!/bin/bash
set -e  # Exit on error

# Ensure credentials are stored (one-time setup)
# xcrun notarytool store-credentials --apple-id "<your-apple-id>" --team-id "<your-team-id>"

cargo build --release

# Code signing
codesign --deep --force --verbose --options runtime --entitlements entitlements.plist --sign "Developer ID Application: Andri Kraemer (9NCXVF3Y67)" target/release/easypaste

# Create component package with pkgbuild
pkgbuild \
  --identifier ch.matlon.easypaste \
  --version 1.0.0 \
  --install-location /usr/local/bin \
  --root dist \
  easypaste.pkg

# Create resources folder with HTML dialogs
mkdir -p resources

cat > resources/welcome.html <<EOL
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Welcome</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif; }
    </style>
</head>
<body>
<h2>Welcome to the easypaste Installer</h2>
<p>This tool installs the <code>easypaste</code> command-line utility to <code>/usr/local/bin</code>.</p>
<p>Once installed, open the Terminal and type <code>easypaste --help</code> to get started.</p>
</body>
</html>
EOL

cat > resources/conclusion.html <<EOL
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Installation Complete</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif; }
    </style>
</head>
<body>
<h2>Installation Complete</h2>
<p><code>easypaste</code> has been installed.</p>
<p>To run it:</p>
<ol>
  <li>Open the Terminal (e.g. via Spotlight Search)</li>
  <li>Type: <code>easypaste</code></li>
</ol>
<p>To uninstall it later, run:</p>
<pre>sudo rm /usr/local/bin/easypaste</pre>
</body>
</html>
EOL

# Create distribution XML
cat > distribution.xml <<EOL
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="1.0">
  <title>easypaste Installer</title>
  <welcome file="welcome.html"/>
  <conclusion file="conclusion.html"/>
  <pkg-ref id="ch.matlon.easypaste"/>
  <options customize="never" allow-external-scripts="no"/>
  <domains enable_anywhere="true"/>

  <choices-outline>
    <line choice="default"/>
  </choices-outline>

  <choice id="default" visible="false">
    <pkg-ref id="ch.matlon.easypaste"/>
  </choice>

  <pkg-ref id="ch.matlon.easypaste" version="1.0.0" auth="Root">easypaste.pkg</pkg-ref>
</installer-gui-script>
EOL

# Create final distribution-style .pkg with productbuild
productbuild \
  --distribution distribution.xml \
  --resources resources \
  --package-path . \
  easypaste-installer-unsigned.pkg

# Sign and notarize
productsign --sign "Developer ID Installer: Andri Kraemer (9NCXVF3Y67)" easypaste-installer-unsigned.pkg easypaste-installer.pkg
sudo sntp -sS time.apple.com
xcrun notarytool submit easypaste-installer.pkg --keychain-profile "notary-gfdev" --wait

xcrun stapler staple easypaste-installer.pkg

# Cleanup
rm -rf target
rm easypaste.spec
rm -rf resources
rm distribution.xml
rm easypaste-installer-unsigned.pkg  # Remove unsigned component .pkg
rm easypaste.pkg  


# Final message
echo ""
echo "✅ Build complete: easypaste-installer.pkg is signed, notarized, and stapled."
echo ""
echo "📦 To install, double-click easypaste-installer.pkg."
echo "💡 After install, open Terminal and run: easypaste"
echo "🧹 To uninstall: sudo rm /usr/local/bin/easypaste"
echo ""
