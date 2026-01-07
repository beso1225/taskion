//
//  ContentView.swift
//  Taskion
//
//  Created by 髙木勇太朗 on 2026/01/07.
//

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var store: AppStore
    
    var body: some View {
        NavigationStack {
            List {
                if store.isLoading {
                    ProgressView("読み込み中...")
                } else if let error = store.errorMessage {
                    Text(error)
                        .foregroundColor(.red)
                } else if store.courses.isEmpty {
                    Text("コースがありません")
                        .foregroundColor(.secondary)
                } else {
                    ForEach(store.courses) { course in
                        NavigationLink(destination: CourseDetailView(course: course)) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(course.title)
                                    .font(.headline)
                                if let day = course.dayOfWeek, let period = course.period {
                                    Text("\(day) \(period)限")
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                            }
                            .padding(.vertical, 4)
                        }
                    }
                }
            }
            .navigationTitle("コース一覧")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button {
                        Task {
                            await store.fetchCourses()
                        }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                    }
                    .disabled(store.isLoading)
                }
            }
            .task {
                await store.fetchCourses()
            }
            .refreshable {
                await store.fetchCourses()
            }
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppStore())
}
