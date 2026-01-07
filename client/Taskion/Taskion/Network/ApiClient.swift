import Foundation

final class ApiClient {
    private let session: URLSession = .shared

    func request<T: Decodable>(_ endpoint: Endpoint, body: Data? = nil) async throws -> T {
        // パスにクエリが含まれる場合は直接URLを構築
        let url: URL
        if endpoint.path.contains("?") {
            url = URL(string: AppConfig.baseURL.absoluteString + endpoint.path)!
        } else {
            url = AppConfig.baseURL.appendingPathComponent(endpoint.path)
        }
        
        var req = URLRequest(url: url)
        req.httpMethod = endpoint.method
        if let body = body {
            req.httpBody = body
            req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }

        let (data, resp) = try await session.data(for: req)
        guard let http = resp as? HTTPURLResponse else {
            throw ApiError.invalidURL
        }
        guard (200..<300).contains(http.statusCode) else {
            throw ApiError.serverStatus(http.statusCode)
        }
        do {
            return try JSONDecoder.api.decode(T.self, from: data)
        } catch {
            throw ApiError.decoding(error)
        }
    }

    // For endpoints that return no content (e.g., 204)
    func requestNoContent(_ endpoint: Endpoint, body: Data? = nil) async throws {
        // パスにクエリが含まれる場合は直接URLを構築
        let url: URL
        if endpoint.path.contains("?") {
            url = URL(string: AppConfig.baseURL.absoluteString + endpoint.path)!
        } else {
            url = AppConfig.baseURL.appendingPathComponent(endpoint.path)
        }
        
        var req = URLRequest(url: url)
        req.httpMethod = endpoint.method
        if let body = body {
            req.httpBody = body
            req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }

        let (_, resp) = try await session.data(for: req)
        guard let http = resp as? HTTPURLResponse else {
            throw ApiError.invalidURL
        }
        guard (200..<300).contains(http.statusCode) else {
            throw ApiError.serverStatus(http.statusCode)
        }
    }
}

extension JSONDecoder {
    static var api: JSONDecoder {
        let d = JSONDecoder()
        return d
    }
}