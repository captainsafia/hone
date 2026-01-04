# Quick Start Guide

Welcome to Hone! This guide will help you get started quickly.

## First Launch

### 1. Build or Download
- **From source**: See [BUILD.md](BUILD.md)
- **Download**: Get the latest release from GitHub

### 2. Launch Hone
Double-click the Hone app. You'll see a pencil icon appear in your menu bar.

### 3. Authenticate with GitHub
1. Click the menu bar icon or press ⌘⇧Space
2. On first launch, you'll see GitHub device authentication
3. Follow the displayed instructions:
   - Visit the provided URL
   - Enter the code shown
   - Authorize Hone
4. Return to Hone - you're authenticated!

## Creating Your First Issue

### Step 1: Open Hone
- **Click** the menu bar icon, or
- **Press** ⌘⇧Space (from anywhere!)

### Step 2: Select Repository
1. In the "Repository" field, type: `owner/repo`
   - Example: `facebook/react` or `microsoft/vscode`
2. Click the "Set" button
3. You'll see "Selected: owner/repo" below

### Step 3: Describe Your Issue
Type naturally in the "Issue Description" box:

```
The app crashes when I click the submit button.

Steps to reproduce:
1. Open the app
2. Fill out the form
3. Click submit
4. App crashes

Expected: Form should submit successfully
Actual: App crashes
```

### Step 4: Add Attachments (Optional)
- **Drag & drop** files into the text area, or
- **Paste** files directly

Files will appear in the attachments list. Click ❌ to remove any file.

### Step 5: Create Issue
Click "Create Issue" button.

Hone will:
1. Upload attachments to a private gist (if any)
2. Structure your text (if LLM configured)
3. Open GitHub in your browser with everything prefilled

### Step 6: Submit on GitHub
Your browser opens to GitHub with:
- ✅ Title filled
- ✅ Body formatted
- ✅ Attachments linked

Review and click "Submit new issue" on GitHub.

Done! 🎉

## Advanced Features

### Adding LLM Support (Optional)

For AI-powered issue structuring, add an API key to Keychain:

#### Option 1: OpenAI
```bash
security add-generic-password -a openai_key -s openai_key -w "your-api-key-here"
```

#### Option 2: Anthropic
```bash
security add-generic-password -a anthropic_key -s anthropic_key -w "your-api-key-here"
```

#### Option 3: Google Gemini
```bash
security add-generic-password -a gemini_key -s gemini_key -w "your-api-key-here"
```

Hone will automatically use whichever key is available.

**Note**: Hone works perfectly without any LLM keys - it will format your text nicely without AI.

## Tips & Tricks

### Keyboard Shortcuts
- **⌘⇧Space**: Open/close Hone from anywhere
- **Escape**: Close the popover
- **Enter**: Submit (when form is complete)

### Writing Issues
- Write naturally - don't worry about formatting
- Include steps to reproduce
- Mention expected vs actual behavior
- LLM will structure it nicely (if configured)

### Attachments
- Supported: Screenshots, logs, images, text files
- Uploaded to private GitHub gists
- Automatically linked in issue body
- Secure and permanent

### Repository Format
Always use: `owner/repo`
- ✅ `rails/rails`
- ✅ `torvalds/linux`
- ❌ `rails` (incomplete)
- ❌ `github.com/rails/rails` (too much)

## Troubleshooting

### Menu bar icon not appearing
- Make sure the app launched successfully
- Check Console.app for errors
- Try relaunching Hone

### Hotkey not working
- Check if another app uses ⌘⇧Space
- Grant accessibility permissions in System Preferences
- System Preferences > Security & Privacy > Privacy > Accessibility

### Authentication failed
- Check internet connection
- Try again - device codes expire
- Visit https://github.com/settings/applications if issues persist

### Issue not opening in browser
- Check repository name format: `owner/repo`
- Ensure repository exists and is accessible
- Try manually: github.com/owner/repo/issues/new

### Attachments not uploading
- Ensure you're authenticated with GitHub
- Check file access permissions
- Verify files aren't too large (>100MB)

## Privacy & Security

- **All local**: Your data stays on your device
- **Keychain**: API keys stored securely
- **Private gists**: Attachments in your private gists only
- **No tracking**: Zero analytics or telemetry

## Getting Help

- 📖 Read the [README](README.md)
- 🔧 Check [BUILD.md](BUILD.md) for build issues
- 🐛 Open an issue on GitHub
- 💡 Check existing issues for solutions

## Next Steps

Now that you're set up:

1. ⭐ Star the repo if you find Hone useful
2. 🐛 Report bugs or request features
3. 🤝 Contribute improvements
4. 📣 Share with others who might find it useful

Happy issue creating! 🚀
