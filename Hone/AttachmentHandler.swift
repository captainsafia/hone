import Foundation

class AttachmentHandler {
    static let shared = AttachmentHandler()
    
    private init() {}
    
    func uploadToGist(files: [URL]) async throws -> [String: String] {
        guard let token = GitHubAuth.shared.getAccessToken() else {
            throw AttachmentError.notAuthenticated
        }
        
        var gistFiles: [String: [String: String]] = [:]
        
        for fileURL in files {
            guard fileURL.startAccessingSecurityScopedResource() else {
                continue
            }
            defer { fileURL.stopAccessingSecurityScopedResource() }
            
            let data = try Data(contentsOf: fileURL)
            let content = data.base64EncodedString()
            let filename = fileURL.lastPathComponent
            
            gistFiles[filename] = ["content": content]
        }
        
        let gistData: [String: Any] = [
            "description": "Hone attachments",
            "public": false,
            "files": gistFiles
        ]
        
        let url = URL(string: "https://api.github.com/gists")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: gistData)
        
        let (data, response) = try await URLSession.shared.data(for: request)
        
        guard let httpResponse = response as? HTTPURLResponse,
              (200...299).contains(httpResponse.statusCode) else {
            throw AttachmentError.uploadFailed
        }
        
        let gistResponse = try JSONDecoder().decode(GistResponse.self, from: data)
        
        var fileURLs: [String: String] = [:]
        for (filename, fileData) in gistResponse.files {
            fileURLs[filename] = fileData.rawUrl
        }
        
        return fileURLs
    }
}

struct GistResponse: Codable {
    let id: String
    let htmlUrl: String
    let files: [String: GistFile]
    
    enum CodingKeys: String, CodingKey {
        case id
        case htmlUrl = "html_url"
        case files
    }
}

struct GistFile: Codable {
    let filename: String
    let rawUrl: String
    
    enum CodingKeys: String, CodingKey {
        case filename
        case rawUrl = "raw_url"
    }
}

enum AttachmentError: LocalizedError {
    case notAuthenticated
    case uploadFailed
    case invalidFile
    
    var errorDescription: String? {
        switch self {
        case .notAuthenticated:
            return "Not authenticated with GitHub"
        case .uploadFailed:
            return "Failed to upload attachments"
        case .invalidFile:
            return "Invalid file"
        }
    }
}
