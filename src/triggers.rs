use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventTapPlacement, CGEventTapOptions, CGEventType};
use parking_lot::Mutex;

/// Debounce interval for activity events (milliseconds).
/// Only one event is sent per this interval to prevent flooding.
const DEBOUNCE_MS: u64 = 200;

#[derive(Debug, Clone)]
pub enum TriggerEvent {
    Activity,
}

pub struct TriggerMonitor {
    running: Arc<AtomicBool>,
}

impl TriggerMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self, tx: Sender<TriggerEvent>) {
        self.running.store(true, Ordering::SeqCst);

        let tx_activity = tx.clone();

        // Thread for Event Tap (Mouse/Keyboard/Click)
        std::thread::spawn(move || {
            let tx = tx_activity;
            let last_event = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(DEBOUNCE_MS)));
            let debounce_duration = Duration::from_millis(DEBOUNCE_MS);
            
            // Monitor mouse movement, mouse clicks, and key down
            let events = vec![
                CGEventType::MouseMoved,
                CGEventType::LeftMouseDown,
                CGEventType::KeyDown,
            ];

            let last_event_clone = last_event.clone();
            let tap = match CGEventTap::new(
                CGEventTapLocation::HID,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                events,
                move |_proxy, _etype, _event| {
                    let now = Instant::now();
                    let mut last = last_event_clone.lock();
                    
                    // Debounce: only send if enough time has passed
                    if now.duration_since(*last) >= debounce_duration {
                        *last = now;
                        let _ = tx.send(TriggerEvent::Activity);
                    }
                    None // Don't block the event
                },
            ) {
                Ok(tap) => tap,
                Err(_) => {
                    log::error!("Failed to create EventTap. Accessibility permissions are required for smart triggers.");
                    return;
                }
            };

            log::info!("Event tap started successfully (debounce: {}ms)", DEBOUNCE_MS);
            unsafe {
                let loop_source = tap.mach_port.create_runloop_source(0).expect("Failed to create runloop source");
                let current_loop = core_foundation::runloop::CFRunLoop::get_current();
                current_loop.add_source(&loop_source, core_foundation::runloop::kCFRunLoopCommonModes);
                tap.enable();
                core_foundation::runloop::CFRunLoop::run_current();
            }
        });

        log::info!("Trigger monitor started");
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

