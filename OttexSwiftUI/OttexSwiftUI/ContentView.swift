import SwiftUI
import UniformTypeIdentifiers

// MARK: - Model Tiers

enum ModelTier: String, CaseIterable, Identifiable {
    case light = "light"
    case standard = "standard"
    case maximum = "maximum"

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .light: return "Light"
        case .standard: return "Standard"
        case .maximum: return "Maximum"
        }
    }

    var repoId: String {
        switch self {
        case .light: return "openai/whisper-base"
        case .standard: return "openai/whisper-small"
        case .maximum: return "openai/whisper-large-v3-turbo"
        }
    }

    var downloadSize: String {
        switch self {
        case .light: return "~150 MB"
        case .standard: return "~1.5 GB"
        case .maximum: return "~3 GB"
        }
    }

    var ramUsage: String {
        switch self {
        case .light: return "~300 MB RAM"
        case .standard: return "~1 GB RAM"
        case .maximum: return "~6 GB RAM"
        }
    }

    var description: String {
        switch self {
        case .light: return "Fast, lower accuracy. Good for simple dictation."
        case .standard: return "Balanced speed and accuracy. Recommended for most users."
        case .maximum: return "Highest accuracy. Best for complex audio and multiple languages."
        }
    }

    var icon: String {
        switch self {
        case .light: return "hare"
        case .standard: return "scalemass"
        case .maximum: return "brain.head.profile"
        }
    }
}

// MARK: - Design System

enum OttexColors {
    static let midnight = Color(red: 0.05, green: 0.06, blue: 0.12)
    static let deepIndigo = Color(red: 0.09, green: 0.10, blue: 0.20)
    static let panel = Color(red: 0.11, green: 0.12, blue: 0.20)
    static let panelRaised = Color(red: 0.14, green: 0.15, blue: 0.24)
    static let amber = Color(red: 0.96, green: 0.62, blue: 0.04)
    static let gold = Color(red: 0.92, green: 0.70, blue: 0.03)
    static let orange = Color(red: 0.92, green: 0.35, blue: 0.05)
    static let coral = Color(red: 0.94, green: 0.42, blue: 0.34)
    static let cyan = Color(red: 0.26, green: 0.75, blue: 0.92)
    static let warmCream = Color(red: 0.98, green: 0.96, blue: 0.92)
    static let mutedText = Color.white.opacity(0.60)
    static let faintText = Color.white.opacity(0.38)
    static let glassBorder = Color.white.opacity(0.12)
    static let glassFill = Color.white.opacity(0.05)
}

struct PanelCard: ViewModifier {
    var cornerRadius: CGFloat = 24
    var padding: CGFloat = 18

    func body(content: Content) -> some View {
        content
            .padding(padding)
            .background(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [
                                OttexColors.panelRaised.opacity(0.92),
                                OttexColors.panel.opacity(0.78)
                            ],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                    .background(
                        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                            .fill(.ultraThinMaterial)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                            .strokeBorder(
                                LinearGradient(
                                    colors: [Color.white.opacity(0.16), Color.white.opacity(0.05)],
                                    startPoint: .topLeading,
                                    endPoint: .bottomTrailing
                                ),
                                lineWidth: 1
                            )
                    )
                    .shadow(color: .black.opacity(0.35), radius: 30, x: 0, y: 18)
            )
    }
}

extension View {
    func panelCard(cornerRadius: CGFloat = 24, padding: CGFloat = 18) -> some View {
        modifier(PanelCard(cornerRadius: cornerRadius, padding: padding))
    }
}

struct ShimmerMeshBackground: View {
    let isRecording: Bool
    let audioLevel: CGFloat

    private struct Orb: Identifiable {
        let id = UUID()
        let xRatio: CGFloat
        let yRatio: CGFloat
        let size: CGFloat
        let drift: CGFloat
        let phase: CGFloat
        let color: Color
    }

    private let orbs: [Orb] = [
        Orb(xRatio: 0.18, yRatio: 0.18, size: 260, drift: 0.18, phase: 0.1, color: OttexColors.amber),
        Orb(xRatio: 0.78, yRatio: 0.20, size: 220, drift: 0.15, phase: 1.2, color: OttexColors.cyan),
        Orb(xRatio: 0.52, yRatio: 0.52, size: 320, drift: 0.10, phase: 2.3, color: OttexColors.gold),
        Orb(xRatio: 0.24, yRatio: 0.78, size: 240, drift: 0.16, phase: 2.8, color: OttexColors.orange),
        Orb(xRatio: 0.84, yRatio: 0.74, size: 280, drift: 0.12, phase: 0.9, color: OttexColors.coral)
    ]

    var body: some View {
        TimelineView(.animation) { timeline in
            let time = timeline.date.timeIntervalSinceReferenceDate

            ZStack {
                LinearGradient(
                    colors: [OttexColors.midnight, OttexColors.deepIndigo, Color.black],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )

                Canvas { context, size in
                    let pulse = isRecording ? 1.0 + audioLevel * 0.32 : 1.0

                    for orb in orbs {
                        let driftX = sin(time * orb.drift + orb.phase) * 42
                        let driftY = cos(time * (orb.drift * 0.9) + orb.phase) * 34
                        let radius = orb.size * pulse
                        let center = CGPoint(
                            x: orb.xRatio * size.width + driftX,
                            y: orb.yRatio * size.height + driftY
                        )
                        let rect = CGRect(x: center.x - radius / 2, y: center.y - radius / 2, width: radius, height: radius)
                        let gradient = Gradient(colors: [
                            orb.color.opacity(isRecording ? 0.34 : 0.24),
                            orb.color.opacity(0.10),
                            .clear
                        ])

                        context.addFilter(.blur(radius: isRecording ? 46 : 58))
                        context.fill(
                            Path(ellipseIn: rect),
                            with: .radialGradient(gradient, center: center, startRadius: 0, endRadius: radius / 2)
                        )
                    }

                    let lineCount = 14
                    for index in 0..<lineCount {
                        let progress = CGFloat(index) / CGFloat(max(lineCount - 1, 1))
                        let y = progress * size.height
                        var path = Path()
                        path.move(to: CGPoint(x: 0, y: y))

                        for step in stride(from: 0.0, through: size.width, by: 18.0) {
                            let phase = CGFloat(time * 0.45) + CGFloat(index) * 0.55
                            let wave = sin((step / 110.0) + phase) * (isRecording ? 14 + audioLevel * 20 : 10)
                            path.addLine(to: CGPoint(x: step, y: y + wave))
                        }

                        context.stroke(
                            path,
                            with: .color(Color.white.opacity(isRecording ? 0.09 : 0.045)),
                            lineWidth: isRecording ? 1.4 : 0.8
                        )
                    }
                }

                Rectangle()
                    .fill(
                        LinearGradient(
                            colors: [Color.black.opacity(0.10), Color.black.opacity(0.34)],
                            startPoint: .top,
                            endPoint: .bottom
                        )
                    )
            }
        }
        .ignoresSafeArea()
    }
}

struct AudioHaloView: View {
    let isRecording: Bool
    let audioLevel: CGFloat

    var body: some View {
        ZStack {
            Circle()
                .fill(
                    RadialGradient(
                        colors: [
                            OttexColors.amber.opacity(isRecording ? 0.28 + Double(audioLevel) * 0.24 : 0.16),
                            OttexColors.orange.opacity(isRecording ? 0.12 : 0.04),
                            .clear
                        ],
                        center: .center,
                        startRadius: 24,
                        endRadius: isRecording ? 130 + audioLevel * 28 : 112
                    )
                )
                .frame(width: 280, height: 280)

            Circle()
                .strokeBorder(
                    AngularGradient(
                        colors: [OttexColors.amber, OttexColors.gold, OttexColors.coral, OttexColors.amber],
                        center: .center
                    ),
                    lineWidth: isRecording ? 5 + audioLevel * 3 : 3
                )
                .frame(width: isRecording ? 134 + audioLevel * 16 : 122, height: isRecording ? 134 + audioLevel * 16 : 122)
                .blur(radius: isRecording ? 0 : 0.2)

            Circle()
                .fill(Color.white.opacity(0.03))
                .frame(width: 106, height: 106)
                .overlay(
                    Circle()
                        .stroke(Color.white.opacity(0.10), lineWidth: 1)
                )

            Image(systemName: isRecording ? "waveform.circle.fill" : "mic.fill")
                .font(.system(size: 44, weight: .medium))
                .foregroundStyle(
                    LinearGradient(colors: [OttexColors.warmCream, OttexColors.amber], startPoint: .top, endPoint: .bottom)
                )
        }
        .animation(.easeInOut(duration: 0.18), value: audioLevel)
        .animation(.spring(response: 0.35, dampingFraction: 0.82), value: isRecording)
    }
}

struct InfoChip: View {
    let title: String
    let value: String
    let accent: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.system(size: 11, weight: .semibold, design: .rounded))
                .textCase(.uppercase)
                .foregroundColor(OttexColors.faintText)

            Text(value)
                .font(.system(size: 14, weight: .semibold, design: .rounded))
                .foregroundColor(OttexColors.warmCream)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(accent.opacity(0.10))
                .overlay(
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .stroke(accent.opacity(0.28), lineWidth: 1)
                )
        )
    }
}

struct ContentView: View {
    @EnvironmentObject var engineWrapper: OttexEngineWrapper
    @State private var showingHistory = false
    @State private var showingSettings = false
    @State private var showingExport = false

    var body: some View {
        ZStack {
            ShimmerMeshBackground(isRecording: engineWrapper.isRecording, audioLevel: engineWrapper.audioLevel)

            HStack(spacing: 24) {
                heroPanel
                sidePanel
            }
            .padding(28)
            .frame(minWidth: 920, idealWidth: 960, maxWidth: 1080, minHeight: 640, idealHeight: 680)
        }
    }

    private var heroPanel: some View {
        VStack(alignment: .leading, spacing: 22) {
            topBar

            VStack(alignment: .leading, spacing: 12) {
                Text("Ambient dictation")
                    .font(.system(size: 13, weight: .semibold, design: .rounded))
                    .textCase(.uppercase)
                    .tracking(1.4)
                    .foregroundColor(OttexColors.faintText)

                Text(engineWrapper.isRecording ? "Listening for your next phrase." : "Speak naturally. Drop text anywhere.")
                    .font(.system(size: 34, weight: .bold, design: .rounded))
                    .foregroundColor(OttexColors.warmCream)
                    .fixedSize(horizontal: false, vertical: true)

                Text("Designed to live in the background and feel calm while idle, alive while recording, and precise when it pastes text into the focused app.")
                    .font(.system(size: 15, weight: .regular, design: .rounded))
                    .foregroundColor(OttexColors.mutedText)
                    .fixedSize(horizontal: false, vertical: true)
            }

            HStack(alignment: .center, spacing: 26) {
                AudioHaloView(isRecording: engineWrapper.isRecording, audioLevel: engineWrapper.audioLevel)
                    .frame(maxWidth: .infinity)

                VStack(alignment: .leading, spacing: 12) {
                    statusRow(title: "State", value: engineWrapper.status, accent: engineWrapper.isRecording ? OttexColors.orange : OttexColors.cyan)
                    statusRow(title: "Hotkey", value: "⌥Space anywhere", accent: OttexColors.amber)
                    statusRow(title: "Insertion", value: "Paste into focused field", accent: OttexColors.gold)

                    if engineWrapper.isRecording {
                        statusRow(title: "Live session", value: engineWrapper.recordingTimeFormatted, accent: OttexColors.coral)
                    } else {
                        statusRow(title: "Model", value: engineWrapper.selectedModelTier.displayName, accent: OttexColors.cyan)
                    }
                }
                .frame(width: 250)
            }
            .frame(maxWidth: .infinity)

            HStack(spacing: 12) {
                InfoChip(title: "Language", value: engineWrapper.selectedLanguage.uppercased(), accent: OttexColors.cyan)
                InfoChip(title: "Mode", value: engineWrapper.isRecording ? "Recording" : "Standby", accent: engineWrapper.isRecording ? OttexColors.orange : OttexColors.amber)
                InfoChip(title: "Transcript", value: engineWrapper.transcribedText.isEmpty ? "Waiting" : "Ready", accent: OttexColors.gold)
            }

            actionArea
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .panelCard(cornerRadius: 30, padding: 24)
    }

    private var sidePanel: some View {
        VStack(alignment: .leading, spacing: 18) {
            transcriptPanel
            historyPreviewPanel
        }
        .frame(width: 360)
        .frame(maxHeight: .infinity)
    }

    private var topBar: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Ottex")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundColor(OttexColors.warmCream)
                Text("Global voice-to-text for macOS")
                    .font(.system(size: 13, weight: .medium, design: .rounded))
                    .foregroundColor(OttexColors.mutedText)
            }

            Spacer()

            toolbarIcon(systemName: "clock.arrow.circlepath", action: { showingHistory.toggle() })
                .popover(isPresented: $showingHistory) {
                    HistoryPopover(engineWrapper: engineWrapper, isPresented: $showingHistory)
                }

            toolbarIcon(systemName: "gearshape", action: { showingSettings.toggle() })
                .popover(isPresented: $showingSettings) {
                    SettingsPopover(engineWrapper: engineWrapper)
                }
        }
    }

    private func toolbarIcon(systemName: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: systemName)
                .font(.system(size: 14, weight: .semibold))
                .foregroundColor(OttexColors.warmCream)
                .frame(width: 34, height: 34)
                .background(
                    Circle()
                        .fill(Color.white.opacity(0.08))
                        .overlay(Circle().stroke(Color.white.opacity(0.10), lineWidth: 1))
                )
        }
        .buttonStyle(.plain)
    }

    private func statusRow(title: String, value: String, accent: Color) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Circle()
                .fill(accent)
                .frame(width: 8, height: 8)
                .padding(.top, 5)

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .textCase(.uppercase)
                    .foregroundColor(OttexColors.faintText)
                Text(value)
                    .font(.system(size: 14, weight: .semibold, design: .rounded))
                    .foregroundColor(OttexColors.warmCream)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(Color.white.opacity(0.05))
                .overlay(
                    RoundedRectangle(cornerRadius: 16, style: .continuous)
                        .stroke(Color.white.opacity(0.08), lineWidth: 1)
                )
        )
    }

    private var actionArea: some View {
        VStack(alignment: .leading, spacing: 14) {
            Button(action: {
                engineWrapper.toggleRecording()
            }) {
                HStack(spacing: 12) {
                    Image(systemName: engineWrapper.isRecording ? "stop.fill" : "mic.fill")
                        .font(.system(size: 15, weight: .bold))
                    Text(engineWrapper.isRecording ? "Stop and Transcribe" : "Start Dictation")
                        .font(.system(size: 16, weight: .bold, design: .rounded))
                    Spacer()
                    Text(engineWrapper.isRecording ? engineWrapper.recordingTimeFormatted : "⌥Space")
                        .font(.system(size: 13, weight: .semibold, design: .rounded))
                        .foregroundColor(Color.white.opacity(0.82))
                }
                .foregroundColor(.white)
                .padding(.horizontal, 18)
                .padding(.vertical, 16)
                .background(
                    RoundedRectangle(cornerRadius: 18, style: .continuous)
                        .fill(
                            LinearGradient(
                                colors: engineWrapper.isRecording
                                    ? [OttexColors.orange, OttexColors.coral]
                                    : [OttexColors.amber, OttexColors.gold],
                                startPoint: .leading,
                                endPoint: .trailing
                            )
                        )
                        .shadow(color: (engineWrapper.isRecording ? OttexColors.coral : OttexColors.amber).opacity(0.42), radius: 18, x: 0, y: 8)
                )
            }
            .buttonStyle(.plain)
            .keyboardShortcut("d", modifiers: .command)
            .disabled(engineWrapper.status.contains("Loading") || engineWrapper.status.contains("Transcribing"))

            Text("Use ⌥Space globally or ⌘D while the app is focused. The focused app receives the final transcript automatically.")
                .font(.system(size: 13, weight: .medium, design: .rounded))
                .foregroundColor(OttexColors.mutedText)
        }
    }

    private var transcriptPanel: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Transcript")
                        .font(.system(size: 18, weight: .bold, design: .rounded))
                        .foregroundColor(OttexColors.warmCream)
                    Text(engineWrapper.wordAndCharCount)
                        .font(.system(size: 12, weight: .medium, design: .rounded))
                        .foregroundColor(OttexColors.faintText)
                }
                Spacer()

                if !engineWrapper.transcribedText.isEmpty {
                    Button(action: { showingExport = true }) {
                        Image(systemName: "square.and.arrow.up")
                            .foregroundColor(OttexColors.warmCream)
                    }
                    .buttonStyle(.plain)
                    .popover(isPresented: $showingExport) {
                        ExportPopover(text: engineWrapper.transcribedText, isPresented: $showingExport)
                    }

                    Button(action: copyTranscription) {
                        Image(systemName: "doc.on.doc")
                            .foregroundColor(OttexColors.warmCream)
                    }
                    .buttonStyle(.plain)
                }
            }

            ZStack(alignment: .topLeading) {
                RoundedRectangle(cornerRadius: 20, style: .continuous)
                    .fill(Color.black.opacity(0.22))
                    .overlay(
                        RoundedRectangle(cornerRadius: 20, style: .continuous)
                            .stroke(Color.white.opacity(0.07), lineWidth: 1)
                    )

                if engineWrapper.transcribedText.isEmpty {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("Your next transcript appears here.")
                            .font(.system(size: 16, weight: .semibold, design: .rounded))
                            .foregroundColor(OttexColors.warmCream.opacity(0.85))
                        Text("Keep any text field focused in another app, trigger dictation, and Ottex will transcribe then paste the result automatically.")
                            .font(.system(size: 13, weight: .medium, design: .rounded))
                            .foregroundColor(OttexColors.mutedText)
                    }
                    .padding(18)
                }

                TextEditor(text: $engineWrapper.transcribedText)
                    .font(.system(size: 14, weight: .regular, design: .rounded))
                    .foregroundColor(OttexColors.warmCream)
                    .scrollContentBackground(.hidden)
                    .padding(14)
                    .background(Color.clear)
            }
            .frame(minHeight: 280, maxHeight: .infinity)
        }
        .panelCard(cornerRadius: 28, padding: 18)
    }

    private var historyPreviewPanel: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Recent")
                .font(.system(size: 16, weight: .bold, design: .rounded))
                .foregroundColor(OttexColors.warmCream)

            if engineWrapper.recentTranscriptions.isEmpty {
                Text("No recent captures yet.")
                    .font(.system(size: 13, weight: .medium, design: .rounded))
                    .foregroundColor(OttexColors.mutedText)
            } else {
                ForEach(Array(engineWrapper.recentTranscriptions.prefix(3))) { entry in
                    Button(action: { engineWrapper.loadTranscription(entry) }) {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(entry.text)
                                .lineLimit(2)
                                .font(.system(size: 13, weight: .medium, design: .rounded))
                                .foregroundColor(OttexColors.warmCream)
                                .multilineTextAlignment(.leading)

                            Text("\(entry.duration)s · \(entry.date.formatted(date: .omitted, time: .shortened))")
                                .font(.system(size: 11, weight: .semibold, design: .rounded))
                                .foregroundColor(OttexColors.faintText)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(12)
                        .background(
                            RoundedRectangle(cornerRadius: 16, style: .continuous)
                                .fill(Color.white.opacity(0.05))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 16, style: .continuous)
                                        .stroke(Color.white.opacity(0.06), lineWidth: 1)
                                )
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .panelCard(cornerRadius: 24, padding: 18)
    }

    private func copyTranscription() {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(engineWrapper.transcribedText, forType: .string)
    }
}

// MARK: - History Popover

struct HistoryPopover: View {
    @ObservedObject var engineWrapper: OttexEngineWrapper
    @Binding var isPresented: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Recent Transcriptions")
                .font(.headline)
                .padding(.bottom, 4)

            if engineWrapper.recentTranscriptions.isEmpty {
                Text("No transcriptions yet")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 20)
            } else {
                ScrollView {
                    VStack(spacing: 8) {
                        ForEach(engineWrapper.recentTranscriptions) { entry in
                            Button(action: {
                                engineWrapper.loadTranscription(entry)
                                isPresented = false
                            }) {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(entry.text)
                                        .lineLimit(3)
                                        .font(.caption)
                                        .foregroundColor(.primary)
                                        .multilineTextAlignment(.leading)

                                    HStack {
                                        Text(entry.date, style: .time)
                                            .font(.caption2)
                                            .foregroundColor(.secondary)
                                        Text("·")
                                            .foregroundColor(.secondary)
                                        Text("\(entry.duration)s")
                                            .font(.caption2)
                                            .foregroundColor(.secondary)
                                    }
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(10)
                                .background(Color.black.opacity(0.05))
                                .cornerRadius(8)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
                .frame(maxHeight: 280)
            }
        }
        .padding(14)
        .frame(width: 280)
    }
}

// MARK: - Settings Popover

struct SettingsPopover: View {
    @ObservedObject var engineWrapper: OttexEngineWrapper
    @State private var showModelChangeConfirm = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Settings")
                .font(.headline)

            HStack {
                Text("Language")
                    .font(.caption)
                Spacer()
                Picker("", selection: $engineWrapper.selectedLanguage) {
                    ForEach(OttexEngineWrapper.supportedLanguages, id: \.code) { lang in
                        Text(lang.name).tag(lang.code)
                    }
                }
                .frame(width: 130)
            }

            Toggle(isOn: $engineWrapper.autoPunctuation) {
                Text("Auto-punctuation")
                    .font(.caption)
            }
            .toggleStyle(.switch)
            .controlSize(.small)

            Divider()

            HStack {
                Text("Model")
                    .font(.caption)
                Spacer()
                Text(engineWrapper.selectedModelTier.displayName)
                    .font(.caption)
                    .foregroundColor(.secondary)
                Button("Change") {
                    showModelChangeConfirm = true
                }
                .font(.caption)
            }

            HStack {
                Text("Global Hotkey")
                    .font(.caption)
                Spacer()
                Text("⌥Space")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(12)
        .frame(width: 280)
        .alert("Change Model?", isPresented: $showModelChangeConfirm) {
            Button("Reset Setup") {
                engineWrapper.hasCompletedSetup = false
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This will show the model selection screen. The new model will be downloaded when you select it.")
        }
    }
}

// MARK: - Export Popover

struct ExportPopover: View {
    let text: String
    @Binding var isPresented: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Export")
                .font(.headline)

            Button(action: { saveTXT() }) {
                Label("Save as .txt", systemImage: "doc.text")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)
            .padding(.vertical, 4)

            Button(action: { copyFormatted() }) {
                Label("Copy formatted text", systemImage: "doc.richtext")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)
            .padding(.vertical, 4)
        }
        .padding(12)
        .frame(width: 220)
    }

    private func saveTXT() {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.plainText]
        panel.nameFieldStringValue = "transcription.txt"
        panel.begin { response in
            if response == .OK, let url = panel.url {
                try? text.write(to: url, atomically: true, encoding: .utf8)
            }
        }
        isPresented = false
    }

    private func copyFormatted() {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)
        isPresented = false
    }
}

// MARK: - Model Setup View (First Run)

struct ModelSetupView: View {
    @EnvironmentObject var engineWrapper: OttexEngineWrapper
    @State private var selectedTier: ModelTier = .standard

    var body: some View {
        ZStack {
            ShimmerMeshBackground(isRecording: false, audioLevel: 0)

            VStack(spacing: 24) {
                Spacer()

                VStack(spacing: 10) {
                    Image(systemName: "waveform.badge.mic")
                        .font(.system(size: 44, weight: .medium))
                        .foregroundStyle(
                            LinearGradient(colors: [OttexColors.amber, OttexColors.gold], startPoint: .topLeading, endPoint: .bottomTrailing)
                        )

                    Text("Welcome to Ottex")
                        .font(.system(size: 30, weight: .bold, design: .rounded))
                        .foregroundColor(OttexColors.warmCream)

                    Text("Choose the Whisper model that matches your speed and accuracy needs.")
                        .font(.system(size: 15, weight: .medium, design: .rounded))
                        .foregroundColor(OttexColors.mutedText)
                }

                VStack(spacing: 12) {
                    ForEach(ModelTier.allCases) { tier in
                        ModelTierCard(tier: tier, isSelected: selectedTier == tier)
                            .onTapGesture { selectedTier = tier }
                    }
                }

                Button(action: {
                    engineWrapper.completeSetup(tier: selectedTier)
                }) {
                    Text("Download & Continue")
                        .font(.system(size: 16, weight: .bold, design: .rounded))
                        .foregroundColor(.white)
                        .padding(.vertical, 16)
                        .frame(maxWidth: .infinity)
                        .background(
                            RoundedRectangle(cornerRadius: 18, style: .continuous)
                                .fill(LinearGradient(colors: [OttexColors.amber, OttexColors.gold], startPoint: .leading, endPoint: .trailing))
                                .shadow(color: OttexColors.amber.opacity(0.35), radius: 18, x: 0, y: 8)
                        )
                }
                .buttonStyle(.plain)

                Spacer()
            }
            .padding(28)
            .frame(width: 560)
            .panelCard(cornerRadius: 32, padding: 28)
        }
    }
}

struct ModelTierCard: View {
    let tier: ModelTier
    let isSelected: Bool

    var body: some View {
        HStack(spacing: 14) {
            Image(systemName: tier.icon)
                .font(.title2)
                .foregroundColor(isSelected ? OttexColors.amber : OttexColors.warmCream.opacity(0.55))
                .frame(width: 34)

            VStack(alignment: .leading, spacing: 4) {
                Text(tier.displayName)
                    .font(.system(size: 16, weight: .bold, design: .rounded))
                    .foregroundColor(OttexColors.warmCream)
                Text(tier.description)
                    .font(.system(size: 12, weight: .medium, design: .rounded))
                    .foregroundColor(OttexColors.mutedText)
                    .lineLimit(2)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 3) {
                Text(tier.downloadSize)
                    .font(.system(size: 12, weight: .semibold, design: .rounded))
                    .foregroundColor(OttexColors.warmCream.opacity(0.72))
                Text(tier.ramUsage)
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .foregroundColor(OttexColors.faintText)
            }
        }
        .padding(16)
        .background(
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .fill(isSelected ? OttexColors.amber.opacity(0.10) : Color.white.opacity(0.04))
                .overlay(
                    RoundedRectangle(cornerRadius: 18, style: .continuous)
                        .stroke(isSelected ? OttexColors.amber.opacity(0.45) : Color.white.opacity(0.08), lineWidth: isSelected ? 1.5 : 1)
                )
        )
    }
}
