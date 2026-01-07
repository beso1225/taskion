import SwiftUI

struct CourseDetailView: View {
    let course: Course
    @EnvironmentObject var store: AppStore
    
    @State private var showingCreate = false
    @State private var newTitle = ""
    @State private var newDueDate = ""
    @State private var newStatus = "未着手"
    @State private var editingTodo: Todo?
    @State private var editTitle = ""
    @State private var editDueDate = ""
    @State private var editStatus = "未着手"
    
    private let statusOptions = ["未着手", "進行中", "最終確認", "完了"]
    
    var body: some View {
        List {
            if store.isLoading && store.todos.isEmpty {
                ProgressView("読み込み中...")
            } else if let error = store.errorMessage {
                Text(error)
                    .foregroundColor(.red)
            } else {
                let items = store.todos.filter { $0.courseId == course.id }
                if items.isEmpty {
                    Text("この授業に紐づく課題はありません")
                        .foregroundColor(.secondary)
                } else {
                    ForEach(items) { todo in
                        HStack {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(todo.title)
                                    .font(.headline)
                                Text("期限: \(todo.dueDate)")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            Spacer()
                            Text(todo.status)
                                .font(.caption)
                                .padding(6)
                                .background(Color.gray.opacity(0.15))
                                .clipShape(RoundedRectangle(cornerRadius: 6))
                        }
                        .padding(.vertical, 4)
                        .swipeActions(edge: .trailing) {
                            Button {
                                Task {
                                    let req = UpdateTodoRequest(title: nil, dueDate: nil, status: "完了")
                                    await store.updateTodo(id: todo.id, request: req)
                                }
                            } label: {
                                Label("完了", systemImage: "checkmark")
                            }
                            .tint(.green)
                            Button {
                                // Open edit sheet with prefilled values
                                editingTodo = todo
                                editTitle = todo.title
                                editDueDate = todo.dueDate
                                editStatus = todo.status
                            } label: {
                                Label("編集", systemImage: "pencil")
                            }
                            .tint(.blue)
                        }
                        .onTapGesture {
                            editingTodo = todo
                            editTitle = todo.title
                            editDueDate = todo.dueDate
                            editStatus = todo.status
                        }
                    }
                }
            }
        }
        .navigationTitle(course.title)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showingCreate = true
                } label: {
                    Image(systemName: "plus")
                }
            }
            ToolbarItem(placement: .navigationBarLeading) {
                Button {
                    Task {
                        await store.triggerSync()
                    }
                } label: {
                    Image(systemName: "arrow.triangle.2.circlepath")
                }
                .disabled(store.isSyncing)
            }
        }
        .task {
            // 初回表示で Todos を取得
            await store.fetchTodos()
        }
        .refreshable {
            await store.fetchTodos()
        }
        .sheet(isPresented: $showingCreate) {
            NavigationStack {
                Form {
                    Section("課題情報") {
                        TextField("タイトル", text: $newTitle)
                        TextField("期限 (YYYY-MM-DD)", text: $newDueDate)
                        Picker("進捗", selection: $newStatus) {
                            ForEach(statusOptions, id: \.self) { s in
                                Text(s).tag(s)
                            }
                        }
                    }
                }
                .navigationTitle("新規課題")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("キャンセル") { showingCreate = false }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("作成") {
                            Task {
                                let req = NewTodoRequest(courseId: course.id, title: newTitle, dueDate: newDueDate, status: newStatus)
                                await store.createTodo(request: req)
                                showingCreate = false
                                // フォーム初期化
                                newTitle = ""
                                newDueDate = ""
                                newStatus = statusOptions.first ?? "未着手"
                            }
                        }
                        .disabled(newTitle.isEmpty || newDueDate.isEmpty)
                    }
                }
            }
        }
        .sheet(item: $editingTodo) { todo in
            NavigationStack {
                Form {
                    Section("課題の編集") {
                        TextField("タイトル", text: $editTitle)
                        TextField("期限 (YYYY-MM-DD)", text: $editDueDate)
                        Picker("進捗", selection: $editStatus) {
                            ForEach(statusOptions, id: \.self) { s in
                                Text(s).tag(s)
                            }
                        }
                    }
                }
                .navigationTitle("編集")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("閉じる") { editingTodo = nil }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("保存") {
                            Task {
                                // Only allow valid status
                                guard statusOptions.contains(editStatus) else { return }
                                let req = UpdateTodoRequest(
                                    title: editTitle.isEmpty ? nil : editTitle,
                                    dueDate: editDueDate.isEmpty ? nil : editDueDate,
                                    status: editStatus
                                )
                                await store.updateTodo(id: todo.id, request: req)
                                editingTodo = nil
                            }
                        }
                    }
                }
            }
        }
    }
}

#Preview {
    CourseDetailView(course: Course(id: "course-1", title: "Algorithms", dayOfWeek: "Mon", period: 1, room: "101", instructor: "Dr. A", isArchived: false, updatedAt: "2026-01-07T00:00:00Z"))
        .environmentObject(AppStore())
}
