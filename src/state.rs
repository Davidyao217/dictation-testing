use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReadinessState {
    Cold = 0,
    Loading = 1,
    Warm = 2,
    Hot = 3,
    Recording = 4,
    Transcribing = 5,
}

impl From<u8> for ReadinessState {
    fn from(v: u8) -> Self {
        match v {
            0 => ReadinessState::Cold,
            1 => ReadinessState::Loading,
            2 => ReadinessState::Warm,
            3 => ReadinessState::Hot,
            4 => ReadinessState::Recording,
            5 => ReadinessState::Transcribing,
            _ => ReadinessState::Cold,
        }
    }
}

#[derive(Clone)]
pub struct StateManager {
    state: Arc<AtomicU8>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(ReadinessState::Cold as u8)),
        }
    }

    pub fn get(&self) -> ReadinessState {
        ReadinessState::from(self.state.load(Ordering::SeqCst))
    }

    pub fn set(&self, state: ReadinessState) {
        self.state.store(state as u8, Ordering::SeqCst);
        log::info!("State transition -> {:?}", state);
    }

    pub fn transition_to_loading(&self) -> bool {
        let current = self.get();
        if current == ReadinessState::Cold {
            self.set(ReadinessState::Loading);
            true
        } else {
            false
        }
    }

    pub fn transition_to_warm(&self) -> bool {
        let current = self.get();
        if current == ReadinessState::Loading {
            self.set(ReadinessState::Warm);
            true
        } else {
            false
        }
    }

    /// Transition to Hot (idle, ready) state after completing work.
    /// This always succeeds regardless of current state.
    pub fn transition_to_idle(&self) {
        self.set(ReadinessState::Hot);
    }

    pub fn transition_to_recording(&self) -> bool {
        self.set(ReadinessState::Recording);
        true
    }

    pub fn transition_to_transcribing(&self) -> bool {
        self.set(ReadinessState::Transcribing);
        true
    }

    pub fn transition_to_cold(&self) {
        self.set(ReadinessState::Cold);
    }

    pub fn is_ready_for_recording(&self) -> bool {
        matches!(
            self.get(),
            ReadinessState::Warm | ReadinessState::Hot
        )
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Hypothesis #6: Initial state should be Cold
    #[test]
    fn test_initial_state_is_cold() {
        let state = StateManager::new();
        assert_eq!(state.get(), ReadinessState::Cold);
    }

    /// Test Hypothesis #6: transition_to_loading only works from Cold
    #[test]
    fn test_transition_to_loading_from_cold() {
        let state = StateManager::new();
        assert!(state.transition_to_loading(), "Should transition from Cold to Loading");
        assert_eq!(state.get(), ReadinessState::Loading);
    }

    /// Test Hypothesis #6: transition_to_loading fails from non-Cold states
    #[test]
    fn test_transition_to_loading_fails_from_wrong_state() {
        let state = StateManager::new();
        state.set(ReadinessState::Hot);
        
        assert!(!state.transition_to_loading(), "Should NOT transition from Hot to Loading");
        assert_eq!(state.get(), ReadinessState::Hot, "State should remain Hot");
    }

    /// Test transition_to_idle works from any state
    #[test]
    fn test_transition_to_idle_from_any_state() {
        let states_to_test = [
            ReadinessState::Cold,
            ReadinessState::Loading,
            ReadinessState::Warm,
            ReadinessState::Recording,
            ReadinessState::Transcribing,
        ];

        for initial_state in states_to_test {
            let state = StateManager::new();
            state.set(initial_state);
            
            state.transition_to_idle();
            assert_eq!(
                state.get(),
                ReadinessState::Hot,
                "Should transition to Hot from {:?}",
                initial_state
            );
        }
    }

    /// Test Hypothesis #6: transition_to_recording always succeeds
    /// This is important because recording should always be possible
    #[test]
    fn test_transition_to_recording_always_succeeds() {
        let states_to_test = [
            ReadinessState::Cold,
            ReadinessState::Loading,
            ReadinessState::Warm,
            ReadinessState::Hot,
            ReadinessState::Transcribing,
        ];

        for initial_state in states_to_test {
            let state = StateManager::new();
            state.set(initial_state);
            
            assert!(
                state.transition_to_recording(),
                "transition_to_recording should succeed from {:?}",
                initial_state
            );
            assert_eq!(state.get(), ReadinessState::Recording);
        }
    }

    /// Test Hypothesis #6: transition_to_transcribing always succeeds
    #[test]
    fn test_transition_to_transcribing_always_succeeds() {
        let state = StateManager::new();
        state.set(ReadinessState::Recording);
        
        assert!(state.transition_to_transcribing());
        assert_eq!(state.get(), ReadinessState::Transcribing);
    }

    /// Test Hypothesis #6: is_ready_for_recording only returns true for Warm or Hot
    #[test]
    fn test_is_ready_for_recording() {
        let state = StateManager::new();
        
        // Should NOT be ready initially (Cold)
        assert!(!state.is_ready_for_recording(), "Cold state should NOT be ready");
        
        // Loading - not ready
        state.set(ReadinessState::Loading);
        assert!(!state.is_ready_for_recording(), "Loading state should NOT be ready");
        
        // Warm - ready
        state.set(ReadinessState::Warm);
        assert!(state.is_ready_for_recording(), "Warm state SHOULD be ready");
        
        // Hot - ready
        state.set(ReadinessState::Hot);
        assert!(state.is_ready_for_recording(), "Hot state SHOULD be ready");
        
        // Recording - not ready (already recording)
        state.set(ReadinessState::Recording);
        assert!(!state.is_ready_for_recording(), "Recording state should NOT be ready");
        
        // Transcribing - not ready
        state.set(ReadinessState::Transcribing);
        assert!(!state.is_ready_for_recording(), "Transcribing state should NOT be ready");
    }

    /// Test the complete happy path flow
    #[test]
    fn test_complete_recording_flow() {
        let state = StateManager::new();
        
        // Start: Cold
        assert_eq!(state.get(), ReadinessState::Cold);
        
        // User triggers activity -> model starts loading
        assert!(state.transition_to_loading());
        assert_eq!(state.get(), ReadinessState::Loading);
        
        // Model loads successfully -> becomes Hot
        state.transition_to_idle();
        assert_eq!(state.get(), ReadinessState::Hot);
        
        // User presses hotkey -> recording
        assert!(state.transition_to_recording());
        assert_eq!(state.get(), ReadinessState::Recording);
        
        // User releases hotkey -> transcribing
        assert!(state.transition_to_transcribing());
        assert_eq!(state.get(), ReadinessState::Transcribing);
        
        // After transcription completes, return to idle (Hot)
        state.transition_to_idle();
        assert_eq!(state.get(), ReadinessState::Hot);
    }

    /// Test that transition_to_cold works from any state
    #[test]
    fn test_transition_to_cold_works_from_any_state() {
        let states_to_test = [
            ReadinessState::Loading,
            ReadinessState::Warm,
            ReadinessState::Hot,
            ReadinessState::Recording,
            ReadinessState::Transcribing,
        ];

        for initial_state in states_to_test {
            let state = StateManager::new();
            state.set(initial_state);
            state.transition_to_cold();
            assert_eq!(
                state.get(),
                ReadinessState::Cold,
                "Should transition to Cold from {:?}",
                initial_state
            );
        }
    }
}
