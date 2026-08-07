import SwiftUI

@main
struct OttexSwiftUIApp: App {
    @StateObject private var engineWrapper = OttexEngineWrapper()

    var body: some Scene {
        WindowGroup {
            Group {
                if engineWrapper.hasCompletedSetup {
                    ContentView()
                } else {
                    ModelSetupView()
                }
            }
            .environmentObject(engineWrapper)
            .frame(minWidth: 400, minHeight: 400)
            .preferredColorScheme(.dark)
        }
        .windowStyle(.hiddenTitleBar)
        .commands {
            CommandGroup(replacing: .appTermination) {
                Button("Quit Ottex") {
                    engineWrapper.prepareForAppTermination()
                    NSApplication.shared.terminate(nil)
                }
                .keyboardShortcut("q")
            }
        }
    }
}
