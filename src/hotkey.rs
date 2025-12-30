use anyhow::Result;
use crossbeam_channel::Sender;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    Pressed,
    Released,
}

pub struct HotkeyHandler {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    hotkey_id: u32,
}

impl HotkeyHandler {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new()?;

        let hotkey = HotKey::new(
            Some(Modifiers::META | Modifiers::SHIFT),
            Code::KeyD,
        );
        let hotkey_id = hotkey.id();

        manager.register(hotkey)?;
        log::info!("Registered hotkey: Cmd+Shift+D");

        Ok(Self {
            manager,
            hotkey,
            hotkey_id,
        })
    }

    pub fn hotkey_id(&self) -> u32 {
        self.hotkey_id
    }

    pub fn listen(tx: Sender<HotkeyEvent>, hotkey_id: u32) {
        let receiver = GlobalHotKeyEvent::receiver();

        std::thread::spawn(move || {
            loop {
                if let Ok(event) = receiver.recv() {
                    if event.id == hotkey_id {
                        let evt = if event.state == global_hotkey::HotKeyState::Pressed {
                            HotkeyEvent::Pressed
                        } else {
                            HotkeyEvent::Released
                        };
                        let _ = tx.send(evt);
                    }
                }
            }
        });
    }
}

impl Drop for HotkeyHandler {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use crossbeam_channel::unbounded;

    /// Test Hypothesis #4: HotKey ID Consistency
    /// Verifies that hotkey ID generation is deterministic for the same key combination
    #[test]
    fn test_hotkey_id_is_consistent() {
        let hotkey1 = HotKey::new(
            Some(Modifiers::META | Modifiers::SHIFT),
            Code::KeyD,
        );
        let hotkey2 = HotKey::new(
            Some(Modifiers::META | Modifiers::SHIFT),
            Code::KeyD,
        );
        
        assert_eq!(
            hotkey1.id(), 
            hotkey2.id(),
            "Hotkey IDs should be consistent for the same key combination"
        );
    }

    /// Test Hypothesis #4: Different keys should have different IDs
    #[test]
    fn test_different_hotkeys_have_different_ids() {
        let hotkey_d = HotKey::new(
            Some(Modifiers::META | Modifiers::SHIFT),
            Code::KeyD,
        );
        let hotkey_e = HotKey::new(
            Some(Modifiers::META | Modifiers::SHIFT),
            Code::KeyE,
        );
        
        assert_ne!(
            hotkey_d.id(), 
            hotkey_e.id(),
            "Different hotkeys should have different IDs"
        );
    }

    /// Test Hypothesis #4: Different modifiers should have different IDs
    #[test]
    fn test_different_modifiers_have_different_ids() {
        let hotkey_meta_shift = HotKey::new(
            Some(Modifiers::META | Modifiers::SHIFT),
            Code::KeyD,
        );
        let hotkey_meta_only = HotKey::new(
            Some(Modifiers::META),
            Code::KeyD,
        );
        
        assert_ne!(
            hotkey_meta_shift.id(), 
            hotkey_meta_only.id(),
            "Different modifier combinations should have different IDs"
        );
    }

    /// Test Hypothesis #3: Channel communication works correctly
    #[test]
    fn test_channel_can_send_and_receive_events() {
        let (tx, rx) = unbounded::<HotkeyEvent>();
        
        // Simulate what the listener thread does
        tx.send(HotkeyEvent::Pressed).expect("Should send pressed event");
        tx.send(HotkeyEvent::Released).expect("Should send released event");
        
        // Simulate what the event loop does  
        let event1 = rx.try_recv().expect("Should receive pressed event");
        assert!(matches!(event1, HotkeyEvent::Pressed));
        
        let event2 = rx.try_recv().expect("Should receive released event");
        assert!(matches!(event2, HotkeyEvent::Released));
    }

    /// Test Hypothesis #3: Channel buffering - events should not be lost
    #[test]
    fn test_channel_buffers_multiple_events() {
        let (tx, rx) = unbounded::<HotkeyEvent>();
        
        // Rapidly send many events
        for _ in 0..100 {
            tx.send(HotkeyEvent::Pressed).unwrap();
            tx.send(HotkeyEvent::Released).unwrap();
        }
        
        // All events should be buffered and receivable
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        
        assert_eq!(count, 200, "All 200 events should be buffered and received");
    }

    /// Test Hypothesis #3: Dropped sender closes channel
    #[test]
    fn test_dropped_sender_disconnects_channel() {
        let (tx, rx) = unbounded::<HotkeyEvent>();
        
        // Drop the sender
        drop(tx);
        
        // Receiver should report disconnected
        assert!(rx.try_recv().is_err(), "Channel should be disconnected when sender is dropped");
    }

    /// Test that hotkey ID is non-zero (sanity check)
    #[test]
    fn test_hotkey_id_is_nonzero() {
        let hotkey = HotKey::new(
            Some(Modifiers::META | Modifiers::SHIFT),
            Code::KeyD,
        );
        
        assert_ne!(hotkey.id(), 0, "Hotkey ID should not be zero");
    }
}
