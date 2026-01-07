import Foundation
import SwiftUI
import Combine

final class AppStore: ObservableObject {
    @Published var courses: [Course] = []
    @Published var todos: [Todo] = []
    @Published var isLoading = false
    @Published var errorMessage: String?
    @Published var isSyncing = false

    private let apiClient = ApiClient()

    // MARK: - Fetch

    func fetchCourses() async {
        isLoading = true
        errorMessage = nil
        do {
            courses = try await apiClient.request(.courses)
            isLoading = false
        } catch {
            errorMessage = "コース取得失敗: \(error.localizedDescription)"
            isLoading = false
        }
    }

    func fetchTodos() async {
        isLoading = true
        errorMessage = nil
        do {
            todos = try await apiClient.request(.todos)
            isLoading = false
        } catch {
            errorMessage = "Todo取得失敗: \(error.localizedDescription)"
            isLoading = false
        }
    }

    // MARK: - Create

    func createTodo(request: NewTodoRequest) async {
        errorMessage = nil
        do {
            let body = try JSONEncoder().encode(request)
            let _: Todo = try await apiClient.request(
                Endpoint(path: "/todos", method: "POST"),
                body: body
            )
            await fetchTodos() // 再取得で一覧を更新
        } catch {
            errorMessage = "Todo作成失敗: \(error.localizedDescription)"
        }
    }

    // MARK: - Update

    func updateTodo(id: String, request: UpdateTodoRequest) async {
        errorMessage = nil
        do {
            let body = try JSONEncoder().encode(request)
            let _: Todo = try await apiClient.request(
                Endpoint.todoUpdate(id: id),
                body: body
            )
            await fetchTodos()
        } catch {
            errorMessage = "Todo更新失敗: \(error.localizedDescription)"
        }
    }

    // MARK: - Archive

    func archiveTodo(id: String) async {
        errorMessage = nil
        do {
            try await apiClient.requestNoContent(Endpoint.todoArchive(id: id))
            await fetchTodos()
        } catch {
            errorMessage = "Todoアーカイブ失敗: \(error.localizedDescription)"
        }
    }

    // MARK: - Sync

    func triggerSync() async {
        isSyncing = true
        errorMessage = nil
        do {
            struct SyncResult: Decodable {}
            let _: SyncResult = try await apiClient.request(.sync())
            // 同期後に再取得
            await fetchCourses()
            await fetchTodos()
            isSyncing = false
        } catch {
            errorMessage = "同期失敗: \(error.localizedDescription)"
            isSyncing = false
        }
    }
}