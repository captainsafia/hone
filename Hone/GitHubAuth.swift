import Foundation

class GitHubAuth {
    static let shared = GitHubAuth()
    
    private let clientId = "Ov23liUbKRqHFaF8lxmC"
    private var accessToken: String?
    
    private init() {
        loadToken()
    }
    
    func isAuthenticated() -> Bool {
        return accessToken != nil
    }
    
    func authenticate() async throws {
        let deviceCodeResponse = try await requestDeviceCode()
        
        print("Please visit: \(deviceCodeResponse.verificationUri)")
        print("Enter code: \(deviceCodeResponse.userCode)")
        
        try await pollForToken(deviceCode: deviceCodeResponse.deviceCode, interval: deviceCodeResponse.interval)
    }
    
    func getAccessToken() -> String? {
        return accessToken
    }
    
    private func requestDeviceCode() async throws -> DeviceCodeResponse {
        let url = URL(string: "https://github.com/login/device/code")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body = ["client_id": clientId]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        let (data, _) = try await URLSession.shared.data(for: request)
        return try JSONDecoder().decode(DeviceCodeResponse.self, from: data)
    }
    
    private func pollForToken(deviceCode: String, interval: Int) async throws {
        let url = URL(string: "https://github.com/login/oauth/access_token")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body = [
            "client_id": clientId,
            "device_code": deviceCode,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
        ]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        for _ in 0..<100 {
            try await Task.sleep(nanoseconds: UInt64(interval) * 1_000_000_000)
            
            let (data, _) = try await URLSession.shared.data(for: request)
            
            if let response = try? JSONDecoder().decode(TokenResponse.self, from: data) {
                accessToken = response.accessToken
                saveToken(response.accessToken)
                return
            }
        }
        
        throw GitHubAuthError.timeout
    }
    
    private func saveToken(_ token: String) {
        KeychainHelper.shared.save(key: "github_token", value: token)
    }
    
    private func loadToken() {
        accessToken = KeychainHelper.shared.load(key: "github_token")
    }
    
    func getUserRepos() async throws -> [GitHubRepo] {
        guard let token = accessToken else {
            throw GitHubAuthError.notAuthenticated
        }
        
        let url = URL(string: "https://api.github.com/user/repos?sort=updated&per_page=100")!
        var request = URLRequest(url: url)
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        
        let (data, _) = try await URLSession.shared.data(for: request)
        return try JSONDecoder().decode([GitHubRepo].self, from: data)
    }
}

struct DeviceCodeResponse: Codable {
    let deviceCode: String
    let userCode: String
    let verificationUri: String
    let interval: Int
    
    enum CodingKeys: String, CodingKey {
        case deviceCode = "device_code"
        case userCode = "user_code"
        case verificationUri = "verification_uri"
        case interval
    }
}

struct TokenResponse: Codable {
    let accessToken: String
    
    enum CodingKeys: String, CodingKey {
        case accessToken = "access_token"
    }
}

struct GitHubRepo: Codable, Identifiable {
    let id: Int
    let name: String
    let fullName: String
    let owner: Owner
    
    struct Owner: Codable {
        let login: String
    }
    
    enum CodingKeys: String, CodingKey {
        case id, name
        case fullName = "full_name"
        case owner
    }
}

enum GitHubAuthError: LocalizedError {
    case notAuthenticated
    case timeout
    
    var errorDescription: String? {
        switch self {
        case .notAuthenticated:
            return "Not authenticated with GitHub"
        case .timeout:
            return "Authentication timed out"
        }
    }
}
