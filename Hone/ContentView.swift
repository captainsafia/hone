import SwiftUI

struct ContentView: View {
    @StateObject private var viewModel = IssueViewModel()
    @State private var issueText: String = ""
    @State private var selectedRepo: String = ""
    @State private var repoInput: String = ""
    @State private var isProcessing = false
    @State private var errorMessage: String?
    @State private var attachments: [URL] = []
    
    var body: some View {
        VStack(spacing: 16) {
            Text("Create GitHub Issue")
                .font(.headline)
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Repository:")
                    .font(.subheadline)
                HStack {
                    TextField("owner/repo", text: $repoInput)
                        .textFieldStyle(.roundedBorder)
                    Button("Set") {
                        selectedRepo = repoInput
                    }
                    .buttonStyle(.borderedProminent)
                }
                if !selectedRepo.isEmpty {
                    Text("Selected: \(selectedRepo)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Issue Description:")
                    .font(.subheadline)
                TextEditor(text: $issueText)
                    .frame(minHeight: 120)
                    .border(Color.gray.opacity(0.3))
                    .onDrop(of: ["public.file-url"], isTargeted: nil) { providers in
                        handleDrop(providers: providers)
                        return true
                    }
            }
            
            if !attachments.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Attachments:")
                        .font(.subheadline)
                    ForEach(attachments, id: \.self) { url in
                        HStack {
                            Text(url.lastPathComponent)
                                .font(.caption)
                            Spacer()
                            Button(action: {
                                attachments.removeAll { $0 == url }
                            }) {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.red)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
                .padding(8)
                .background(Color.gray.opacity(0.1))
                .cornerRadius(4)
            }
            
            if let error = errorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundColor(.red)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            
            HStack {
                Button("Cancel") {
                    closePopover()
                }
                .keyboardShortcut(.escape)
                
                Spacer()
                
                Button(isProcessing ? "Processing..." : "Create Issue") {
                    createIssue()
                }
                .buttonStyle(.borderedProminent)
                .disabled(issueText.isEmpty || selectedRepo.isEmpty || isProcessing)
            }
        }
        .padding()
        .frame(width: 450, height: 450)
        .onAppear {
            viewModel.checkAuthStatus()
        }
    }
    
    private func handleDrop(providers: [NSItemProvider]) {
        for provider in providers {
            provider.loadItem(forTypeIdentifier: "public.file-url", options: nil) { (item, error) in
                if let data = item as? Data,
                   let url = URL(dataRepresentation: data, relativeTo: nil) {
                    DispatchQueue.main.async {
                        attachments.append(url)
                    }
                }
            }
        }
    }
    
    private func createIssue() {
        guard !selectedRepo.isEmpty, !issueText.isEmpty else { return }
        
        isProcessing = true
        errorMessage = nil
        
        Task {
            do {
                let attachmentURLs = try await viewModel.uploadAttachments(attachments)
                let (title, body) = try await viewModel.structureIssue(text: issueText, attachmentURLs: attachmentURLs)
                await viewModel.openGitHubIssue(repo: selectedRepo, title: title, body: body)
                
                await MainActor.run {
                    issueText = ""
                    selectedRepo = ""
                    repoInput = ""
                    attachments = []
                    isProcessing = false
                    closePopover()
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    isProcessing = false
                }
            }
        }
    }
    
    private func closePopover() {
        if let delegate = NSApplication.shared.delegate as? AppDelegate {
            delegate.menuBarController?.closePopover()
        }
    }
}

#Preview {
    ContentView()
}
