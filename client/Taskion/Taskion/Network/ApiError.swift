import Foundation

enum ApiError: Error {
    case invalidURL
    case transport(Error)
    case serverStatus(Int)
    case decoding(Error)
}

extension ApiError: LocalizedError {
    var errorDescription: String? {
        switch self {
        case .invalidURL:
            return "不正なURLです"
        case .transport(let underlying):
            return underlying.localizedDescription
        case .serverStatus(let status):
            return "HTTPステータス異常: \(status)"
        case .decoding(let underlying):
            return "JSONデコード失敗: \(underlying.localizedDescription)"
        }
    }
}