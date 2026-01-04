# Contributing to Hone

Thank you for your interest in contributing to Hone! This document provides guidelines and information for contributors.

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on what is best for the community

## How to Contribute

### Reporting Issues

If you find a bug or have a feature request:

1. Check if the issue already exists
2. If not, create a new issue with:
   - Clear title and description
   - Steps to reproduce (for bugs)
   - Expected vs actual behavior
   - macOS version and Hone version
   - Relevant logs or screenshots

### Submitting Pull Requests

1. Fork the repository
2. Create a new branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Test thoroughly
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to your branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Pull Request Guidelines

- Follow the existing code style
- Add comments for complex logic
- Update documentation if needed
- Test on macOS before submitting
- Keep PRs focused on a single feature or fix

## Development Setup

See [BUILD.md](BUILD.md) for detailed build instructions.

## Project Structure

- `HoneApp.swift` - Application entry point and lifecycle
- `ContentView.swift` - Main user interface
- `MenuBarController.swift` - Menu bar icon and popover management
- `IssueViewModel.swift` - Business logic and state management
- `GitHubAuth.swift` - GitHub OAuth device flow implementation
- `AttachmentHandler.swift` - File upload to GitHub gists
- `LLMService.swift` - AI-powered issue structuring
- `KeychainHelper.swift` - Secure credential storage
- `HotkeyManager.swift` - Global hotkey registration

## Coding Standards

### Swift Style

- Use Swift 5.0+ features
- Follow Apple's Swift API Design Guidelines
- Use meaningful variable and function names
- Add documentation comments for public APIs

### UI/UX

- Keep the UI simple and focused
- Maintain consistency with macOS design patterns
- Test with different system appearance settings (light/dark mode)
- Ensure keyboard navigation works properly

### Security

- Never commit API keys or tokens
- Use Keychain for sensitive data
- Validate all user inputs
- Follow secure coding practices

## Testing

Before submitting a PR:

1. Test basic functionality:
   - Menu bar icon appears
   - Popover opens/closes
   - Hotkey works (⌘⇧Space)
   
2. Test GitHub integration:
   - OAuth flow completes
   - Repository selection works
   - Issue creation succeeds
   
3. Test attachments:
   - Drag and drop works
   - Paste works
   - Files upload to gists
   
4. Test LLM integration (if keys configured):
   - Issue text is structured properly
   - Fallback works without keys

## Adding New Features

When adding new features:

1. Discuss the feature in an issue first
2. Keep changes minimal and focused
3. Update README.md if user-facing
4. Consider graceful degradation
5. Test edge cases

## Common Tasks

### Adding a New LLM Provider

1. Add case to `LLMProvider` enum in `LLMService.swift`
2. Implement `structureWith[Provider]` method
3. Add keychain key handling
4. Update documentation

### Modifying the UI

1. Make changes in `ContentView.swift`
2. Test with different window sizes
3. Ensure dark mode compatibility
4. Verify keyboard shortcuts still work

### Changing OAuth Flow

1. Update `GitHubAuth.swift`
2. Test authentication flow
3. Verify token storage/retrieval
4. Check token refresh if applicable

## Getting Help

- Open an issue for questions
- Check existing issues and PRs
- Review the code and documentation

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
