enum ApiError: Error {
    case invalidURL
    case transport(Error)
    case serverStatus(Int)
    case decoding(Error)
}