import Foundation

/// How a long dictation recording is cut for transcription (OpenBot nightly t3-b2cd3129).
///
/// One request holds at most 90 seconds of audio, and a part ends at the quietest half-second near
/// that limit so a word is not cut in two. A recording runs for at most ten minutes. Everything here
/// is pure: the split and the loudness of a piece can be tested without a microphone or a provider.
enum DictationSplit {
    /// A dictation records for up to ten minutes; the pill stops at this point.
    static let maxBytes = MicTap.bytesPerSecond * 60 * 10
    /// The longest part sent to a speech-to-text provider.
    static let partBytes = MicTap.bytesPerSecond * 90
    /// How far back from the limit a quiet point is looked for.
    private static let lookBackBytes = MicTap.bytesPerSecond * 10
    /// A quiet point is half a second of the quietest audio near the limit.
    private static let quietBytes = MicTap.bytesPerSecond / 2
    /// A tail under half a second stays with the part before it.
    private static let minTailBytes = MicTap.bytesPerSecond / 2
    /// Never make a part shorter than this just to find a quiet point.
    private static let minPartBytes = MicTap.bytesPerSecond * 45
    /// Only a genuinely quiet window moves a cut back from the limit; steady speech waits.
    private static let quietLevel = 0.15

    /// Byte ranges of a recording of `count` bytes. `loudness` gives the mean amplitude, 0…1, of a
    /// range of PCM; a lower value is a quieter point to split at.
    static func ranges(of count: Int, loudness: (Range<Int>) -> Double) -> [Range<Int>] {
        guard count > 0 else { return [] }
        var parts: [Range<Int>] = []
        var start = 0
        while count - start > partBytes {
            let limit = start + partBytes
            let window = max(start + minPartBytes, limit - lookBackBytes)
            var cut = limit
            var quietest = Double.infinity
            for probe in stride(from: window, to: limit, by: quietBytes) {
                let level = loudness(probe..<min(probe + quietBytes, limit))
                if level < Self.quietLevel, level < quietest {
                    quietest = level
                    cut = probe
                }
            }
            parts.append(start..<cut)
            start = cut
        }
        if start < count { parts.append(start..<count) }
        // A very short tail would make the provider invent a word for what is barely a tap.
        if parts.count > 1, let last = parts.last, count - last.lowerBound < minTailBytes {
            parts[parts.count - 2] = parts[parts.count - 2].lowerBound..<count
            parts.removeLast()
        }
        return parts
    }

    /// Mean amplitude of 16 kHz mono PCM16 bytes, 0…1. No audio is read by the split itself.
    static func level(of pcm: Data) -> Double {
        let samples = pcm.count / 2
        guard samples > 0 else { return 0 }
        var sum = 0.0
        pcm.withUnsafeBytes { raw in
            for i in 0..<samples {
                let value = Double(raw.loadUnaligned(fromByteOffset: i * 2, as: Int16.self)) / 32_768
                sum += value * value
            }
        }
        return (sum / Double(samples)).squareRoot()
    }
}
