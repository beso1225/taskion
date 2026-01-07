import Foundation

final class ApiClient {
    private let session: URLSession = .shared

    func request<T: Decodable>(_ endpoint: Endpoint, body: Data? = nil) async throws -> T {
        let url = AppConfig.baseURL.appendingPathComponent(endpoint.path)
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
}

extension JSONDecoder {
    static var api: JSONDecoder {
        let d = JSONDecoder()
        return d
    }
}