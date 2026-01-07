//
//  TaskionApp.swift
//  Taskion
//
//  Created by 髙木勇太朗 on 2026/01/07.
//

import SwiftUI

@main
struct TaskionApp: App {
    @StateObject private var store = AppStore()
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(store)
        }
    }
}
