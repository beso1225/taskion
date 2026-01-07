import Foundation

struct Todo: Codable, Identifiable {
    let id: String
    let courseId: String
    let title: String
    let dueDate: String
    let status: String
    let completedAt: String?
    let isArchived: Bool
    let updatedAt: String
    let syncState: String
    let lastSyncedAt: String?

    enum CodingKeys: String, CodingKey {
        case id
        case courseId = "course_id"
        case title
        case dueDate = "due_date"
        case status
        case completedAt = "completed_at"
        case isArchived = "is_archived"
        case updatedAt = "updated_at"
        case syncState = "sync_state"
        case lastSyncedAt = "last_synced_at"
    }
}

struct NewTodoRequest: Codable {
    let courseId: String
    let title: String
    let dueDate: String
    let status: String

    enum CodingKeys: String, CodingKey {
        case courseId = "course_id"
        case title
        case dueDate = "due_date"
        case status
    }
}

struct UpdateTodoRequest: Codable {
    let title: String?
    let dueDate: String?
    let status: String?

    enum CodingKeys: String, CodingKey {
        case title
        case dueDate = "due_date"
        case status
    }
}