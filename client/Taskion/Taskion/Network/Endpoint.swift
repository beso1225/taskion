struct Endpoint {
    let path: String
    let method: String // "GET" | "POST" | "PATCH"
}

extension Endpoint {
    static let health = Endpoint(path: "/health", method: "GET")
    static let courses = Endpoint(path: "/courses", method: "GET")
    static let todos = Endpoint(path: "/todos", method: "GET")
    static func sync() -> Endpoint { Endpoint(path: "/sync", method: "POST") }
    static func todoUpdate(id: String) -> Endpoint { Endpoint(path: "/todos/\(id)", method: "PATCH") }
}