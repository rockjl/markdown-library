/// Abstract interface for Automatic Speech Recognition providers.
///
/// Implementations:
/// - `VoiceEngine` (Xunfei RTASR WebSocket)
/// - Whisper (planned)
/// - Azure Speech (planned)
pub trait TranscriptProvider {
    /// Start listening for speech input.
    fn start(&mut self);

    /// Stop listening and finalise the current transcription.
    fn stop(&mut self);

    /// Retrieve the latest transcription text.
    fn latest_transcript(&self) -> String;
}
