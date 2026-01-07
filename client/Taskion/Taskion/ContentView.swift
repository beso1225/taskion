//
//  ContentView.swift
//  Taskion
//
//  Created by 髙木勇太朗 on 2026/01/07.
//

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var store: AppStore
    @AppStorage("defaultSemester") private var selectedSemester: String = "すべて"
    
    // セメスターリストを計算
    private var semesters: [String] {
        let allSemesters = store.courses.flatMap { course in
            course.semester.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        }
        let uniqueSemesters = Array(Set(allSemesters)).sorted()
        return ["すべて"] + uniqueSemesters
    }
    
    // セメスターの優先順位（Sが先、Aが後）
    private func semesterPriority(_ semester: String) -> Int {
        let semesters = semester.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        // 最初のセメスターで判定
        if let first = semesters.first {
            // 学年を抽出（1S1 → 1, 2A1 → 2）
            let yearMatch = first.prefix(1)
            let year = Int(yearMatch) ?? 0
            
            // S/Aを判定
            let termType = first.contains("S") ? 0 : (first.contains("A") ? 1 : 2)
            
            // 学年 * 100 + ターム種別で優先順位
            return year * 100 + termType
        }
        return 9999
    }
    
    // 曜日の優先順位（英語・日本語両対応）
    private func dayOfWeekPriority(_ dayOfWeek: String) -> Int {
        let dayMap: [String: Int] = [
            "月": 0, "Mon": 0, "Monday": 0,
            "火": 1, "Tue": 1, "Tuesday": 1,
            "水": 2, "Wed": 2, "Wednesday": 2,
            "木": 3, "Thu": 3, "Thursday": 3,
            "金": 4, "Fri": 4, "Friday": 4,
            "土": 5, "Sat": 5, "Saturday": 5,
            "日": 6, "Sun": 6, "Sunday": 6
        ]
        return dayMap[dayOfWeek] ?? 7
    }
    
    // フィルタリング＆ソートされたコース
    private var filteredCourses: [Course] {
        let filtered: [Course]
        if selectedSemester == "すべて" {
            filtered = store.courses
        } else {
            filtered = store.courses.filter { course in
                let courseSemesters = course.semester.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
                return courseSemesters.contains(selectedSemester)
            }
        }
        
        let sorted = filtered.sorted { c1, c2 in
            // 1. セメスター順（S→A）
            let s1 = semesterPriority(c1.semester)
            let s2 = semesterPriority(c2.semester)
            if s1 != s2 { return s1 < s2 }
            
            // 2. 曜日順
            let d1 = dayOfWeekPriority(c1.dayOfWeek)
            let d2 = dayOfWeekPriority(c2.dayOfWeek)
            if d1 != d2 { return d1 < d2 }
            
            // 3. 時限順
            return c1.period < c2.period
        }
        
        // デバッグ出力
        for course in sorted {
            print("Course: \(course.title), Semester: \(course.semester), Day: \(course.dayOfWeek), Period: \(course.period)")
        }
        
        return sorted
    }
    
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // セメスター選択ピッカー
                if !semesters.isEmpty && semesters.count > 1 {
                    Picker("セメスター", selection: $selectedSemester) {
                        ForEach(semesters, id: \.self) { semester in
                            Text(semester).tag(semester)
                        }
                    }
                    .pickerStyle(.segmented)
                    .padding()
                }
                
                List {
                    if store.isLoading {
                        ProgressView("読み込み中...")
                    } else if let error = store.errorMessage {
                        Text(error)
                            .foregroundColor(.red)
                    } else if filteredCourses.isEmpty {
                        Text(selectedSemester == "すべて" ? "コースがありません" : "該当するコースがありません")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(filteredCourses) { course in
                            NavigationLink(destination: CourseDetailView(course: course)) {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(course.title)
                                        .font(.headline)
                                    Text("\(course.dayOfWeek) \(course.period)限")
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                                .padding(.vertical, 4)
                            }
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
