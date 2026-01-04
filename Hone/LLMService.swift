import Foundation

enum LLMProvider: String, CaseIterable {
    case openai = "OpenAI"
    case anthropic = "Anthropic"
    case gemini = "Gemini"
}

class LLMService {
    static let shared = LLMService()
    
    private init() {}
    
    func structureIssue(text: String, attachmentURLs: [String: String]) async throws -> (title: String, body: String) {
        let provider = getConfiguredProvider()
        
        switch provider {
        case .openai:
            return try await structureWithOpenAI(text: text, attachmentURLs: attachmentURLs)
        case .anthropic:
            return try await structureWithAnthropic(text: text, attachmentURLs: attachmentURLs)
        case .gemini:
            return try await structureWithGemini(text: text, attachmentURLs: attachmentURLs)
        }
    }
    
    private func getConfiguredProvider() -> LLMProvider {
        if KeychainHelper.shared.load(key: "openai_key") != nil {
            return .openai
        } else if KeychainHelper.shared.load(key: "anthropic_key") != nil {
            return .anthropic
        } else if KeychainHelper.shared.load(key: "gemini_key") != nil {
            return .gemini
        }
        return .openai
    }
    
    private func structureWithOpenAI(text: String, attachmentURLs: [String: String]) async throws -> (title: String, body: String) {
        guard let apiKey = KeychainHelper.shared.load(key: "openai_key") else {
            return fallbackStructure(text: text, attachmentURLs: attachmentURLs)
        }
        
        let prompt = buildPrompt(text: text, attachmentURLs: attachmentURLs)
        
        let url = URL(string: "https://api.openai.com/v1/chat/completions")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body: [String: Any] = [
            "model": "gpt-4o-mini",
            "messages": [
                ["role": "system", "content": "You are a helpful assistant that structures freeform text into well-formatted GitHub issues. Return only JSON with 'title' and 'body' fields."],
                ["role": "user", "content": prompt]
            ],
            "response_format": ["type": "json_object"]
        ]
        
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        do {
            let (data, _) = try await URLSession.shared.data(for: request)
            let response = try JSONDecoder().decode(OpenAIResponse.self, from: data)
            
            if let content = response.choices.first?.message.content,
               let jsonData = content.data(using: .utf8),
               let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: String],
               let title = json["title"],
               let body = json["body"] {
                return (title, body)
            }
        } catch {
            print("OpenAI API error: \(error)")
        }
        
        return fallbackStructure(text: text, attachmentURLs: attachmentURLs)
    }
    
    private func structureWithAnthropic(text: String, attachmentURLs: [String: String]) async throws -> (title: String, body: String) {
        guard let apiKey = KeychainHelper.shared.load(key: "anthropic_key") else {
            return fallbackStructure(text: text, attachmentURLs: attachmentURLs)
        }
        
        let prompt = buildPrompt(text: text, attachmentURLs: attachmentURLs)
        
        let url = URL(string: "https://api.anthropic.com/v1/messages")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue(apiKey, forHTTPHeaderField: "x-api-key")
        request.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body: [String: Any] = [
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [
                ["role": "user", "content": prompt]
            ]
        ]
        
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        do {
            let (data, _) = try await URLSession.shared.data(for: request)
            let response = try JSONDecoder().decode(AnthropicResponse.self, from: data)
            
            if let content = response.content.first?.text {
                return parseStructuredText(content)
            }
        } catch {
            print("Anthropic API error: \(error)")
        }
        
        return fallbackStructure(text: text, attachmentURLs: attachmentURLs)
    }
    
    private func structureWithGemini(text: String, attachmentURLs: [String: String]) async throws -> (title: String, body: String) {
        guard let apiKey = KeychainHelper.shared.load(key: "gemini_key") else {
            return fallbackStructure(text: text, attachmentURLs: attachmentURLs)
        }
        
        let prompt = buildPrompt(text: text, attachmentURLs: attachmentURLs)
        
        let url = URL(string: "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key=\(apiKey)")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body: [String: Any] = [
            "contents": [
                ["parts": [["text": prompt]]]
            ]
        ]
        
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        do {
            let (data, _) = try await URLSession.shared.data(for: request)
            let response = try JSONDecoder().decode(GeminiResponse.self, from: data)
            
            if let content = response.candidates.first?.content.parts.first?.text {
                return parseStructuredText(content)
            }
        } catch {
            print("Gemini API error: \(error)")
        }
        
        return fallbackStructure(text: text, attachmentURLs: attachmentURLs)
    }
    
    private func buildPrompt(text: String, attachmentURLs: [String: String]) -> String {
        var prompt = """
        Structure the following freeform text into a GitHub issue with a clear title and body.
        Return JSON with 'title' and 'body' fields.
        
        Text: \(text)
        """
        
        if !attachmentURLs.isEmpty {
            prompt += "\n\nAttachments:\n"
            for (filename, url) in attachmentURLs {
                prompt += "- [\(filename)](\(url))\n"
            }
        }
        
        return prompt
    }
    
    private func parseStructuredText(_ text: String) -> (title: String, body: String) {
        if let jsonData = text.data(using: .utf8),
           let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: String],
           let title = json["title"],
           let body = json["body"] {
            return (title, body)
        }
        
        let lines = text.components(separatedBy: .newlines)
        let title = lines.first ?? "Issue"
        let body = lines.dropFirst().joined(separator: "\n")
        return (title, body)
    }
    
    private func fallbackStructure(text: String, attachmentURLs: [String: String]) -> (title: String, body: String) {
        let lines = text.components(separatedBy: .newlines).filter { !$0.isEmpty }
        let title = lines.first ?? "New Issue"
        var body = lines.dropFirst().joined(separator: "\n\n")
        
        if !attachmentURLs.isEmpty {
            body += "\n\n## Attachments\n"
            for (filename, url) in attachmentURLs {
                body += "- [\(filename)](\(url))\n"
            }
        }
        
        return (title, body)
    }
}

struct OpenAIResponse: Codable {
    let choices: [Choice]
    
    struct Choice: Codable {
        let message: Message
    }
    
    struct Message: Codable {
        let content: String
    }
}

struct AnthropicResponse: Codable {
    let content: [Content]
    
    struct Content: Codable {
        let text: String
    }
}

struct GeminiResponse: Codable {
    let candidates: [Candidate]
    
    struct Candidate: Codable {
        let content: Content
    }
    
    struct Content: Codable {
        let parts: [Part]
    }
    
    struct Part: Codable {
        let text: String
    }
}
