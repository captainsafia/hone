# Build Instructions for Hone

## Prerequisites

- macOS 13.0 or later
- Xcode 15.0 or later
- An Apple Developer account (for code signing, optional for local development)

## Building on macOS

### 1. Install Xcode

If you haven't already, install Xcode from the Mac App Store or download it from [developer.apple.com](https://developer.apple.com/xcode/).

### 2. Clone the Repository

```bash
git clone https://github.com/captainsafia/hone.git
cd hone
```

### 3. Open the Project

```bash
open Hone.xcodeproj
```

### 4. Configure Code Signing (Optional for local development)

For local development and testing:
1. Select the Hone project in the navigator
2. Select the Hone target
3. Go to "Signing & Capabilities"
4. Select "Automatically manage signing"
5. Choose your development team (or leave blank for local-only builds)

### 5. Build and Run

Press ⌘R or select Product > Run from the menu.

The app will build and launch. You'll see a pencil icon appear in your menu bar.

## Building from Command Line

```bash
cd /path/to/hone
xcodebuild -project Hone.xcodeproj -scheme Hone -configuration Release build
```

The built app will be in:
```
build/Release/Hone.app
```

## Creating a Release Build

For distribution:

1. Archive the app:
   ```bash
   xcodebuild -project Hone.xcodeproj -scheme Hone -configuration Release archive -archivePath ./build/Hone.xcarchive
   ```

2. Export the archive:
   ```bash
   xcodebuild -exportArchive -archivePath ./build/Hone.xcarchive -exportPath ./build/Release -exportOptionsPlist ExportOptions.plist
   ```

## Troubleshooting

### Code Signing Issues

If you encounter code signing issues:
- Make sure you have Xcode command line tools installed: `xcode-select --install`
- For local testing, you can disable code signing in the build settings (not recommended for distribution)

### Missing Dependencies

All dependencies are standard macOS frameworks included with Xcode. No additional package managers (CocoaPods, Carthage, SPM packages) are required for the base build.

### Sparkle Integration

For auto-updates to work:
1. Add the Sparkle framework to your project
2. Generate an EdDSA key pair for signing updates
3. Update the `SUPublicEDKey` in Info.plist
4. Host an appcast.xml file at the URL specified in Info.plist

## Development Notes

### Project Structure

```
Hone/
├── HoneApp.swift              # App entry point
├── ContentView.swift          # Main UI
├── MenuBarController.swift    # Menu bar management
├── IssueViewModel.swift       # Business logic
├── GitHubAuth.swift          # OAuth implementation
├── AttachmentHandler.swift   # File upload to gists
├── LLMService.swift          # AI structuring
├── KeychainHelper.swift      # Secure storage
├── HotkeyManager.swift       # Global hotkey
├── Info.plist                # App configuration
├── Hone.entitlements        # Capabilities
└── Assets.xcassets/         # Images and icons
```

### Key Technologies

- **SwiftUI**: Modern declarative UI
- **AppKit**: Menu bar integration
- **Carbon**: Global hotkey registration
- **URLSession**: Network requests
- **Security**: Keychain access

## Next Steps

After building:
1. Test basic functionality
2. Configure GitHub OAuth (using the default client ID or your own)
3. Add LLM API keys if desired
4. Test attachment uploads
5. Verify hotkey functionality (⌘⇧Space)
