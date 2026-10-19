// Recording a voice note and playing one back. The microphone is only touched when the person
// presses Start Recording; a refusal is explained in plain words and offers an audio file instead.
// The recording is written to a temporary file, handed to the core as it is, and the file is
// removed afterwards.

import AVFoundation
import Foundation
import Observation
import OpenFrameCore

/// A finished recording on disk.
struct RecordedAudio: Equatable {
    var fileURL: URL
    var durationMs: Double
    var mediaType: String
    var fileExtension: String
}

/// What the recorder needs from the microphone, so tests can stand in a file.
@MainActor
protocol AudioCapturing: AnyObject {
    /// True when the person has allowed (or now allows) the microphone.
    func requestPermission() async -> Bool
    /// Starts recording. Throws a plain message when no input can be used.
    func begin() throws
    var elapsed: TimeInterval { get }
    func finish() throws -> RecordedAudio
    func cancel()
}

enum MicrophoneAccess {
    case allowed, notAsked, denied, unavailable
}

/// The real microphone through AVAudioRecorder (AAC in an .m4a file).
@MainActor
final class MicrophoneCapture: AudioCapturing {
    private var recorder: AVAudioRecorder?
    private var url: URL?

    static var access: MicrophoneAccess {
        // A build without the microphone usage text would be stopped by the system the moment it
        // asked, so it never asks.
        guard Bundle.main.object(forInfoDictionaryKey: "NSMicrophoneUsageDescription") != nil else { return .unavailable }
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized: return .allowed
        case .notDetermined: return .notAsked
        default: return .denied
        }
    }

    func requestPermission() async -> Bool {
        switch Self.access {
        case .allowed: return true
        case .notAsked: return await AVCaptureDevice.requestAccess(for: .audio)
        case .denied, .unavailable: return false
        }
    }

    func begin() throws {
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("openframe-voice-\(UUID().uuidString).m4a")
        let settings: [String: Any] = [
            AVFormatIDKey: kAudioFormatMPEG4AAC,
            AVSampleRateKey: 44_100,
            AVNumberOfChannelsKey: 1,
            AVEncoderBitRateKey: 96_000,
        ]
        let r = try AVAudioRecorder(url: file, settings: settings)
        guard r.prepareToRecord(), r.record() else {
            throw CoreFailure(code: "media.no_input", message: "OpenFrame could not start recording. Check that a microphone is connected.")
        }
        recorder = r
        url = file
    }

    var elapsed: TimeInterval { recorder?.currentTime ?? 0 }

    func finish() throws -> RecordedAudio {
        guard let r = recorder, let file = url else {
            throw CoreFailure(code: "media.not_recording", message: "Nothing was recorded.")
        }
        let seconds = r.currentTime
        r.stop()
        recorder = nil
        url = nil
        return RecordedAudio(fileURL: file, durationMs: (seconds * 1000).rounded(), mediaType: "audio/mp4", fileExtension: "m4a")
    }

    func cancel() {
        recorder?.stop()
        if let url { try? FileManager.default.removeItem(at: url) }
        recorder = nil
        url = nil
    }
}

@MainActor
@Observable
final class VoiceNoteModel {
    enum Phase: Equatable {
        case idle
        case asking
        case recording
        case stopped
        case saving
        /// The microphone cannot be used. `canOpenSettings` is true when the person can change it.
        case blocked(message: String, canOpenSettings: Bool)
        case failed(String)
    }

    private(set) var phase: Phase = .idle
    private(set) var elapsed: TimeInterval = 0
    var name = ""
    private(set) var error: String?

    @ObservationIgnored private let capture: AudioCapturing
    @ObservationIgnored private var recorded: RecordedAudio?
    @ObservationIgnored private var ticker: Task<Void, Never>?

    init(capture: AudioCapturing = MicrophoneCapture()) {
        self.capture = capture
    }

    var statusText: String {
        switch phase {
        case .idle: "Press Start Recording when you are ready."
        case .asking: "Waiting for your answer about the microphone…"
        case .recording: "Recording…"
        case .stopped: "Recording stopped"
        case .saving: "Saving…"
        case .blocked, .failed: ""
        }
    }

    var timeText: String { VaultText.duration(ms: elapsed * 1000) ?? "0:00" }
    var canStart: Bool { if case .idle = phase { true } else if case .failed = phase { true } else { false } }
    var canSave: Bool { phase == .recording || phase == .stopped }

    func start() async {
        error = nil
        phase = .asking
        guard await capture.requestPermission() else {
            let access = MicrophoneCapture.access
            if access == .unavailable, capture is MicrophoneCapture {
                phase = .blocked(message: "This build of OpenFrame cannot ask for the microphone. You can add an audio file instead.", canOpenSettings: false)
            } else {
                phase = .blocked(
                    message: "OpenFrame cannot use the microphone. Allow it in System Settings, under Privacy & Security, Microphone. You can add an audio file instead.",
                    canOpenSettings: true)
            }
            return
        }
        do {
            try capture.begin()
            phase = .recording
            startTicker()
        } catch {
            phase = .failed(CoreFailure(asCore: error).message)
        }
    }

    private func startTicker() {
        ticker?.cancel()
        ticker = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { return }
                if self.phase == .recording { self.elapsed = self.capture.elapsed }
                try? await Task.sleep(for: .milliseconds(250))
            }
        }
    }

    /// Stops the recording and keeps it in the Vault. A failed save keeps the recording so Save
    /// can be tried again.
    func save(into vault: IdeaVaultModel) async -> VaultItemDto? {
        guard canSave || recorded != nil else { return nil }
        error = nil
        ticker?.cancel()
        if recorded == nil {
            do {
                recorded = try capture.finish()
            } catch {
                self.error = CoreFailure(asCore: error).message
                phase = .failed(self.error ?? "")
                return nil
            }
        }
        guard let rec = recorded else { return nil }
        elapsed = rec.durationMs / 1000
        phase = .saving
        do {
            let data = try Data(contentsOf: rec.fileURL)
            guard !data.isEmpty else { throw CoreFailure(code: "media.empty", message: "Nothing was recorded.") }
            let item = try await vault.addVoiceNote(
                data: data, fileName: "Voice note \(Self.stamp()).\(rec.fileExtension)", mediaType: rec.mediaType,
                durationMs: rec.durationMs, title: name)
            try? FileManager.default.removeItem(at: rec.fileURL)
            recorded = nil
            return item
        } catch {
            self.error = CoreFailure(asCore: error).message
            phase = .stopped
            return nil
        }
    }

    /// Drops an unsaved recording.
    func discard() {
        ticker?.cancel()
        capture.cancel()
        if let rec = recorded { try? FileManager.default.removeItem(at: rec.fileURL) }
        recorded = nil
    }

    static func stamp(_ date: Date = Date()) -> String {
        let c = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute], from: date)
        return String(format: "%04d%02d%02d-%02d%02d", c.year ?? 0, c.month ?? 0, c.day ?? 0, c.hour ?? 0, c.minute ?? 0)
    }
}

/// Plays an audio item from its managed file.
@MainActor
@Observable
final class AudioPlayerModel: NSObject, AVAudioPlayerDelegate {
    private(set) var duration: TimeInterval = 0
    private(set) var position: TimeInterval = 0
    private(set) var isPlaying = false
    private(set) var failed = false
    private(set) var loaded = false

    @ObservationIgnored private var player: AVAudioPlayer?
    @ObservationIgnored private var ticker: Task<Void, Never>?

    func load(_ url: URL) {
        stop()
        do {
            let p = try AVAudioPlayer(contentsOf: url)
            p.delegate = self
            p.prepareToPlay()
            player = p
            duration = p.duration
            position = 0
            failed = false
            loaded = true
        } catch {
            player = nil
            failed = true
            loaded = true
        }
    }

    func toggle() {
        guard let player else { return }
        if player.isPlaying {
            player.pause()
            isPlaying = false
        } else {
            player.play()
            isPlaying = true
            startTicker()
        }
    }

    func seek(to seconds: TimeInterval) {
        guard let player else { return }
        player.currentTime = max(0, min(seconds, duration))
        position = player.currentTime
    }

    func stop() {
        ticker?.cancel()
        player?.stop()
        player = nil
        isPlaying = false
        position = 0
    }

    private func startTicker() {
        ticker?.cancel()
        ticker = Task { [weak self] in
            while !Task.isCancelled {
                guard let self, let p = self.player else { return }
                self.position = p.currentTime
                try? await Task.sleep(for: .milliseconds(200))
            }
        }
    }

    nonisolated func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        Task { @MainActor in
            self.isPlaying = false
            self.position = 0
            self.player?.currentTime = 0
        }
    }
}
