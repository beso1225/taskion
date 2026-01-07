import SwiftUI

struct CourseDetailView: View {
    let course: Course
    @EnvironmentObject var store: AppStore

    @State private var showingCreate = false
    @State private var newTitle = ""
    @State private var newDueDate = ""
    @State private var newDueDateDate = Date()
    @State private var newStatus = "未着手"
    @State private var editingTodo: Todo?
    @State private var editTitle = ""
    @State private var editDueDate = ""
    @State private var editDueDateDate = Date()
    @State private var editStatus = "未着手"

    @State private var selectedStatus = "すべて"
    @State private var showArchived = false

    private let statusOptions = ["未着手", "進行中", "最終確認", "完了"]
    private let filterOptions = ["すべて", "未着手", "進行中", "最終確認", "完了"]

    private var showArchivedBinding: Binding<Bool> {
        Binding(
            get: { self.showArchived },
            set: { newValue in
                var txn = Transaction()
                txn.disablesAnimations = newValue
                withTransaction(txn) {
                    self.showArchived = newValue
                }
            }
        )
    }

    // 期限切れ判定
    private func isOverdue(_ dueDate: String) -> Bool {
        let today = formatDate(Date())
        return dueDate < today
    }

    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "ja_JP")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: date)
    }

    private func parseDate(_ string: String) -> Date {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "ja_JP")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.date(from: string) ?? Date()
    }

    private var filteredTodos: [Todo] {
        let courseTodos = store.todos.filter { $0.courseId == course.id }
        let archivedFiltered = showArchived ? courseTodos : courseTodos.filter { !$0.isArchived }
        if selectedStatus == "すべて" {
            return archivedFiltered.sorted { $0.dueDate < $1.dueDate }
        }
        let statusFiltered = archivedFiltered.filter { $0.status == selectedStatus }
        return statusFiltered.sorted { $0.dueDate < $1.dueDate }
    }

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 8) {
                Picker("ステータス", selection: $selectedStatus) {
                    ForEach(filterOptions, id: \.self) { status in
                        Text(status).tag(status)
                    }
                }
                .pickerStyle(.segmented)

                Toggle("アーカイブ済みを表示", isOn: showArchivedBinding)
                    .toggleStyle(.switch)
            }
            .padding()

            List {
                if store.isLoading && store.todos.isEmpty {
                    ProgressView("読み込み中...")
                } else if let error = store.errorMessage {
                    Text(error)
                        .foregroundColor(.red)
                } else if filteredTodos.isEmpty {
                    Text("該当する課題はありません")
                        .foregroundColor(.secondary)
                } else {
                    ForEach(filteredTodos) { todo in
                        todoRow(for: todo)
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
            await store.fetchTodos(includeArchived: showArchived)
        }
        .refreshable {
            await store.fetchTodos(includeArchived: showArchived)
        }
        .onChange(of: showArchived) { _, newValue in
            Task {
                await store.fetchTodos(includeArchived: newValue)
            }
        }
        .sheet(isPresented: $showingCreate) {
            NavigationStack {
                Form {
                    Section("課題情報") {
                        TextField("タイトル", text: $newTitle)
                        DatePicker("期限", selection: $newDueDateDate, displayedComponents: .date)
                        Picker("進捗", selection: $newStatus) {
                            ForEach(statusOptions, id: \.self) { status in
                                Text(status).tag(status)
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
                                let req = NewTodoRequest(courseId: course.id, title: newTitle, dueDate: formatDate(newDueDateDate), status: newStatus)
                                await store.createTodo(request: req)
                                showingCreate = false
                                newTitle = ""
                                newDueDate = ""
                                newDueDateDate = Date()
                                newStatus = statusOptions.first ?? "未着手"
                            }
                        }
                        .disabled(newTitle.isEmpty)
                    }
                }
            }
        }
        .sheet(item: $editingTodo) { todo in
            NavigationStack {
                Form {
                    Section("課題の編集") {
                        TextField("タイトル", text: $editTitle)
                        DatePicker("期限", selection: $editDueDateDate, displayedComponents: .date)
                        Picker("進捗", selection: $editStatus) {
                            ForEach(statusOptions, id: \.self) { status in
                                Text(status).tag(status)
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
                                guard statusOptions.contains(editStatus) else { return }
                                let req = UpdateTodoRequest(
                                    title: editTitle.isEmpty ? nil : editTitle,
                                    dueDate: formatDate(editDueDateDate),
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
        .onChange(of: editingTodo) { _, newValue in
            if let t = newValue {
                editDueDateDate = parseDate(t.dueDate)
            }
        }
    }

    @ViewBuilder
    private func todoRow(for todo: Todo) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(todo.title)
                        .font(.headline)
                    if todo.isArchived {
                        Text("アーカイブ済み")
                            .font(.caption2)
                            .padding(4)
                            .background(Color.gray.opacity(0.2))
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }
                }
                HStack {
                    Text("期限: \(todo.dueDate)")
                        .font(.caption)
                    if isOverdue(todo.dueDate) && todo.status != "完了" {
                        Text("期限切れ")
                            .font(.caption2)
                            .foregroundColor(.white)
                            .padding(4)
                            .background(Color.red)
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }
                }
                .foregroundColor(isOverdue(todo.dueDate) && todo.status != "完了" ? .red : .secondary)
            }
            Spacer()
            Text(todo.status)
                .font(.caption)
                .padding(6)
                .background(Color.gray.opacity(0.15))
                .clipShape(RoundedRectangle(cornerRadius: 6))
        }
        .padding(.vertical, 4)
        .opacity(todo.isArchived ? 0.5 : 1.0)
        .swipeActions(edge: .trailing) {
            if !todo.isArchived {
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
                    editingTodo = todo
                    editTitle = todo.title
                    editDueDate = todo.dueDate
                    editStatus = todo.status
                } label: {
                    Label("編集", systemImage: "pencil")
                }
                .tint(.blue)
            }
        }
        .swipeActions(edge: .leading) {
            if todo.isArchived {
                Button {
                    Task {
                        await store.unarchiveTodo(id: todo.id)
                        if showArchived {
                            await store.fetchTodos(includeArchived: true)
                        }
                    }
                } label: {
                    Label("アーカイブ解除", systemImage: "tray.and.arrow.up")
                }
                .tint(.blue)
            } else {
                Button(role: .destructive) {
                    Task {
                        await store.archiveTodo(id: todo.id)
                        if showArchived {
                            await store.fetchTodos(includeArchived: true)
                        }
                    }
                } label: {
                    Label("アーカイブ", systemImage: "archivebox")
                }
            }
        }
        .onTapGesture {
            if !todo.isArchived {
                editingTodo = todo
                editTitle = todo.title
                editDueDate = todo.dueDate
                editStatus = todo.status
            }
        }
    }
}

#Preview {
    CourseDetailView(course: Course(id: "course-1", title: "Algorithms", semester: "1S1", dayOfWeek: "Mon", period: 1, room: "101", instructor: "Dr. A", isArchived: false, updatedAt: "2026-01-07T00:00:00Z"))
        .environmentObject(AppStore())
}
