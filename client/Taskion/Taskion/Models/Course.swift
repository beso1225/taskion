import Foundation

struct Course: Codable, Identifiable {
    let id: String
    let title: String
    let dayOfWeek: String?
    let period: Int?
    let room: String?
    let instructor: String?
    let isArchived: Bool
    let updatedAt: String

    enum CodingKeys: String, CodingKey {
        case id, title
        case dayOfWeek = "day_of_week"
        case period, room, instructor
        case isArchived = "is_archived"
        case updatedAt = "updated_at"
    }
}