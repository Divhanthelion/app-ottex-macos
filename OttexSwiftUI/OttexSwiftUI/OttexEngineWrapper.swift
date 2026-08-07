import SwiftUI
import Foundation
import Carbon
import CoreGraphics
import ApplicationServices

struct TranscriptionEntry: Identifiable {
    let id = UUID()
    let text: String
    let date: Date
    let duration: Int // seconds
}

// MARK: - Global Hotkey Manager

import Cocoa
import Carbon

class GlobalHotkeyManager {
    private weak var engineWrapper: OttexEngineWrapper?
    private var retryTimer: Timer?
    private var carbonEventHandler: EventHandlerRef?
    private var hotKeyRef: EventHotKeyRef?

    init(engineWrapper: OttexEngineWrapper) {
        self.engineWrapper = engineWrapper
    }

    func start() -> Bool {
        // Accessibility is required for text insertion and microphone is prompted on first record.
        let trusted = AXIsProcessTrusted()
        NSLog("[Ottex Hotkey] AXIsProcessTrusted = \(trusted)")

        if !trusted {
            let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue(): true] as CFDictionary
            AXIsProcessTrustedWithOptions(options)
            NSLog("[Ottex Hotkey] Prompted for accessibility permission, will retry...")
            startRetryTimer()
        }

        setupCarbonHotkey()
        return trusted
    }

    private func startRetryTimer() {
        retryTimer?.invalidate()
        retryTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] timer in
            if AXIsProcessTrusted() {
                NSLog("[Ottex Hotkey] Accessibility permission granted instantly!")
                timer.invalidate()
                self?.retryTimer = nil
                DispatchQueue.main.async {
                    self?.engineWrapper?.accessibilityGranted = true
                }
            }
        }
    }

    private func setupCarbonHotkey() {
        unregisterCarbonHotkey()

        // Register Option+Space (Toggle) via Carbon so we don't need Input Monitoring.
        var eventType = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))

        let ptr = Unmanaged.passUnretained(self).toOpaque()

        InstallEventHandler(GetApplicationEventTarget(), { (nextHandler, theEvent, userData) -> OSStatus in
            guard let userData = userData else { return noErr }
            let manager = Unmanaged<GlobalHotkeyManager>.fromOpaque(userData).takeUnretainedValue()

            NSLog("[Ottex Hotkey] Carbon Option+Space triggered — toggling recording")
            DispatchQueue.main.async {
                manager.engineWrapper?.toggleRecording()
            }
            return noErr
        }, 1, &eventType, ptr, &carbonEventHandler)

        let hotKeyId = EventHotKeyID(signature: 1, id: 1)

        // 49 is Space. optionKey mask is 2048.
        RegisterEventHotKey(
            49,
            UInt32(optionKey),
            hotKeyId,
            GetApplicationEventTarget(),
            0,
            &hotKeyRef
        )
    }

    private func unregisterCarbonHotkey() {
        if let hotKeyRef {
            UnregisterEventHotKey(hotKeyRef)
            self.hotKeyRef = nil
        }
        if let carbonEventHandler {
            RemoveEventHandler(carbonEventHandler)
            self.carbonEventHandler = nil
        }
    }

    func stop() {
        retryTimer?.invalidate()
        retryTimer = nil
        unregisterCarbonHotkey()
    }

    deinit {
        stop()
    }
}

// MARK: - Engine Wrapper

@MainActor
class OttexEngineWrapper: ObservableObject {
    private var engine: OttexEngine?
    private var hotkeyManager: GlobalHotkeyManager?

    @Published var isRecording: Bool = false
    @Published var transcribedText: String = ""
    @Published var status: String = "Waiting for setup..."
    @Published var audioLevel: CGFloat = 0.0
    @Published var recordingSeconds: Int = 0
    @Published var accessibilityGranted: Bool = false

    // Persisted setup state
    @AppStorage("hasCompletedSetup") var hasCompletedSetup: Bool = false
    @AppStorage("selectedModelTier") var selectedModelTierRaw: String = "standard"

    var selectedModelTier: ModelTier {
        ModelTier(rawValue: selectedModelTierRaw) ?? .standard
    }

    // Recent transcriptions
    @Published var recentTranscriptions: [TranscriptionEntry] = []

    // Settings
    @Published var selectedLanguage: String = "en"
    @Published var autoPunctuation: Bool = true
    @Published var appearance: Appearance = .system

    enum Appearance: String, CaseIterable {
        case system = "System"
        case light = "Light"
        case dark = "Dark"
    }

    static let supportedLanguages: [(code: String, name: String)] = [
        ("en", "English"),
        ("es", "Spanish"),
        ("fr", "French"),
        ("de", "German"),
        ("it", "Italian"),
        ("pt", "Portuguese"),
        ("ja", "Japanese"),
        ("ko", "Korean"),
        ("zh", "Chinese"),
        ("ru", "Russian"),
        ("ar", "Arabic"),
        ("hi", "Hindi"),
    ]

    private var recordingTimer: Timer?
    private var waveformTimer: Timer?

    var recordingTimeFormatted: String {
        let mins = recordingSeconds / 60
        let secs = recordingSeconds % 60
        return String(format: "%02d:%02d", mins, secs)
    }

    var wordAndCharCount: String {
        let text = transcribedText
        let chars = text.count
        let words = text.split(whereSeparator: { $0.isWhitespace }).count
        if chars == 0 { return "0 words, 0 characters" }
        return "\(words) word\(words == 1 ? "" : "s"), \(chars) character\(chars == 1 ? "" : "s")"
    }

    var colorScheme: ColorScheme? {
        switch appearance {
        case .system: return nil
        case .light: return .light
        case .dark: return .dark
        }
    }

    init() {
        initLogging()
        // Only auto-start if setup was previously completed
        if hasCompletedSetup {
            let tier = ModelTier(rawValue: selectedModelTierRaw) ?? .standard
            setupEngine(repoId: tier.repoId, revision: "main")
        }
    }

    /// Called from ModelSetupView when user picks a tier
    func completeSetup(tier: ModelTier) {
        selectedModelTierRaw = tier.rawValue
        hasCompletedSetup = true
        setupEngine(repoId: tier.repoId, revision: "main")
    }

    private func setupEngine(repoId: String, revision: String) {
        Task {
            do {
                let config = EngineConfig(
                    whisperRepoId: repoId,
                    whisperRevision: revision
                )

                self.engine = OttexEngine(config: config)
                self.status = "Loading Whisper (Metal GPU)..."

                try await self.engine?.loadModels()
                self.status = "Ready"
                self.registerGlobalHotkey()
            } catch {
                self.status = "Error: \(error.localizedDescription)"
            }
        }
    }

    private func registerGlobalHotkey() {
        let manager = GlobalHotkeyManager(engineWrapper: self)
        self.accessibilityGranted = manager.start()
        self.hotkeyManager = manager
    }

    func toggleRecording() {
        if isRecording {
            stopRecordingAndTranscribe()
        } else {
            startRecording()
        }
    }

    func startRecording() {
        guard !isRecording else { return }
        do {
            try engine?.startRecording()
            isRecording = true
            status = "Recording..."
            recordingSeconds = 0
            startTimers()
        } catch {
            status = "Failed to start recording: \(error)"
        }
    }

    func stopRecordingAndTranscribe() {
        guard isRecording else { return }
        stopTimers()
        let duration = recordingSeconds

        Task {
            do {
                self.isRecording = false
                self.status = "Transcribing..."
                self.audioLevel = 0.0

                if let text = try await engine?.stopAndTranscribe() {
                    self.transcribedText = text
                    self.status = "Ready"

                    // Save to recent transcriptions
                    if !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        let entry = TranscriptionEntry(text: text, date: Date(), duration: duration)
                        self.recentTranscriptions.insert(entry, at: 0)
                        // Keep last 20
                        if self.recentTranscriptions.count > 20 {
                            self.recentTranscriptions.removeLast()
                        }
                    }

                    try self.engine?.typeText(text: text)
                }
            } catch {
                self.status = "Transcription failed: \(error)"
            }
        }
    }

    func prepareForAppTermination() {
        hotkeyManager?.stop()
        hotkeyManager = nil
        stopTimers()

        if isRecording {
            isRecording = false
            audioLevel = 0.0
            status = "Stopping..."
            // Best-effort async stop; quit should not block on transcription.
            let engine = self.engine
            Task {
                _ = try? await engine?.stopAndTranscribe()
            }
        }
    }

    func loadTranscription(_ entry: TranscriptionEntry) {
        transcribedText = entry.text
    }

    func exportAsText() -> String {
        return transcribedText
    }

    // MARK: - Timers

    private func startTimers() {
        recordingTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.recordingSeconds += 1
            }
        }

        waveformTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            Task { @MainActor in
                let target = CGFloat.random(in: 0.3...1.0)
                if let self = self {
                    self.audioLevel = self.audioLevel * 0.7 + target * 0.3
                }
            }
        }
    }

    private func stopTimers() {
        recordingTimer?.invalidate()
        recordingTimer = nil
        waveformTimer?.invalidate()
        waveformTimer = nil
    }
}
