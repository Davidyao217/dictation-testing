/// Events sent to the main event loop from background threads
#[derive(Debug)]
pub enum AppEvent {
    /// Transcription completed successfully with the transcribed text
    TranscriptionComplete(String),
    /// Transcription failed (no speech detected, or inference error)
    TranscriptionFailed,
    /// Quit requested from tray menu
    Quit,
}
