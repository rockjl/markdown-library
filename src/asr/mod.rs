/// Abstract interface for ASR (Automatic Speech Recognition) providers.
///
/// Implementations:
/// - VoiceEngine (Xunfei)
/// - Whisper (future)
/// - Azure Speech (future)
pub trait TranscriptProvider {
    fn start(&mut self);
    fn stop(&mut self);
    fn latest_transcript(&self) -> String;
}
