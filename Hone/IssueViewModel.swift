import Foundation
import AppKit

@MainActor
class IssueViewModel: ObservableObject {
    @Published var isAuthenticated = false
    @Published var errorMessage: String?
    
    func checkAuthStatus() {
        isAuthenticated = GitHubAuth.shared.isAuthenticated()
        
        if !isAuthenticated {
            Task {
                do {
                    try await GitHubAuth.shared.authenticate()
                    isAuthenticated = true
                } catch {
                    errorMessage = "Authentication failed: \(error.localizedDescription)"
                }
            }
        }
    }
    
    func uploadAttachments(_ files: [URL]) async throws -> [String: String] {
        guard !files.isEmpty else { return [:] }
        return try await AttachmentHandler.shared.uploadToGist(files: files)
    }
    
    func structureIssue(text: String, attachmentURLs: [String: String]) async throws -> (title: String, body: String) {
        return try await LLMService.shared.structureIssue(text: text, attachmentURLs: attachmentURLs)
    }
    
    func openGitHubIssue(repo: String, title: String, body: String) {
        let encodedTitle = title.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? ""
        let encodedBody = body.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? ""
        
        let urlString = "https://github.com/\(repo)/issues/new?title=\(encodedTitle)&body=\(encodedBody)"
        
        if let url = URL(string: urlString) {
            NSWorkspace.shared.open(url)
        }
    }
}
