# Hone

A macOS menu bar app for creating GitHub issues with ease.

## Features

- **Menu Bar Integration**: Quick access from the macOS menu bar
- **Global Hotkey**: Press ⌘⇧Space to open Hone from anywhere
- **Freeform Text Input**: Type your issue description naturally
- **Repository Selection**: Choose which GitHub repository to create the issue in
- **Attachment Support**: Drag & drop or paste files to attach to your issue
- **GitHub OAuth**: Secure authentication using GitHub's device flow
- **LLM Enhancement**: Automatically structure your text into a polished GitHub issue
  - Supports OpenAI (GPT-4)
  - Supports Anthropic (Claude)
  - Supports Google Gemini
- **Browser Integration**: Opens GitHub with prefilled issue title and body
- **Graceful Degradation**: Works without LLM keys, using basic formatting
- **Auto-Updates**: Powered by Sparkle framework

## Requirements

- macOS 13.0 or later
- Xcode 15.0 or later (for building from source)

## Setup

### Building from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/captainsafia/hone.git
   cd hone
   ```

2. Open the project in Xcode:
   ```bash
   open Hone.xcodeproj
   ```

3. Build and run the project (⌘R)

### First Run

1. When you first launch Hone, it will appear in your menu bar with a pencil icon
2. Click the icon or press ⌘⇧Space to open the popover
3. You'll be prompted to authenticate with GitHub using the device flow
4. Follow the instructions to complete authentication

### Configuring LLM Keys (Optional)

To enable AI-powered issue structuring, you can add API keys for:

- **OpenAI**: Add key `openai_key` to macOS Keychain
- **Anthropic**: Add key `anthropic_key` to macOS Keychain  
- **Gemini**: Add key `gemini_key` to macOS Keychain

Keys are stored securely in the macOS Keychain and accessed only when needed.

## Usage

1. Click the menu bar icon or press ⌘⇧Space
2. Enter your GitHub repository (format: `owner/repo`)
3. Type your issue description in freeform text
4. Optionally drag & drop or paste attachments
5. Click "Create Issue"
6. Your default browser will open with the issue prefilled in GitHub

## Architecture

- **SwiftUI + AppKit**: Modern UI with system integration
- **GitHub OAuth Device Flow**: Secure authentication without client secrets
- **Secret Gists**: Attachments are uploaded to private GitHub gists
- **Keychain**: API keys and tokens stored securely
- **Carbon Framework**: Global hotkey registration
- **Sparkle**: Automatic update support

## Privacy

- All data stays on your device
- API keys stored in macOS Keychain
- Attachments uploaded to your private GitHub gists
- No analytics or tracking

## License

MIT

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.