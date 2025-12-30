use cocoa::appkit::{NSBackingStoreType, NSColor, NSScreen, NSView, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct RecordingIndicator {
    window: id,
    is_visible: Arc<AtomicBool>,
}

impl RecordingIndicator {
    pub fn new() -> Self {
        unsafe {
            let main_screen = NSScreen::mainScreen(nil);
            // Use visibleFrame to respect Dock and Menu Bar
            let visible_frame: NSRect = msg_send![main_screen, visibleFrame];
            
            let width = 60.0;
            let height = 8.0;
            let margin_bottom = 12.0; // Distance from bottom of visible area
            
            // Calculate center x
            let x = visible_frame.origin.x + (visible_frame.size.width - width) / 2.0;
            // Calculate bottom y (Cocoa origin is bottom-left)
            let y = visible_frame.origin.y + margin_bottom;
            
            let rect = NSRect::new(
                NSPoint::new(x, y),
                NSSize::new(width, height),
            );

            let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
                rect,
                NSWindowStyleMask::NSBorderlessWindowMask,
                NSBackingStoreType::NSBackingStoreBuffered,
                NO,
            );

            // Ensure window is above everything including Dock in some cases, though visibleFrame avoids Dock usually.
            // Level 100 is kCGStatusWindowLevel (25) equivalent roughly or similar. 
            // 24 (NSStatusWindowLevel) is previously used. Let's stick to 25 or 24.
            let _: () = msg_send![window, setLevel: 25i32]; 
            let _: () = msg_send![window, setOpaque: NO];
            let _: () = msg_send![window, setHasShadow: NO]; // We render our own layer shadow for glow
            
            let clear_color = NSColor::clearColor(nil);
            let _: () = msg_send![window, setBackgroundColor: clear_color];
            let _: () = msg_send![window, setIgnoresMouseEvents: YES];
            // NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorTransient
            let _: () = msg_send![window, setCollectionBehavior: 1u64 << 0 | 1u64 << 6];

            let content_view: id = window.contentView();
            let _: () = msg_send![content_view, setWantsLayer: YES];
            
            let layer: id = msg_send![content_view, layer];
            
            // Initial color (Recording Red default?)
            let red_color = NSColor::colorWithRed_green_blue_alpha_(nil, 1.0, 0.3, 0.3, 1.0);
            let cg_color: id = msg_send![red_color, CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg_color];
            let _: () = msg_send![layer, setCornerRadius: height / 2.0];
            
            // Glow effect
            let _: () = msg_send![layer, setShadowOpacity: 0.8f32];
            let _: () = msg_send![layer, setShadowRadius: 8.0f64];
            let shadow_offset = NSSize::new(0.0, 0.0); // Center shadow for glow
            let _: () = msg_send![layer, setShadowOffset: shadow_offset];
            
            let _: () = msg_send![layer, setShadowColor: cg_color]; // Glow matches color

            Self {
                window,
                is_visible: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    pub fn show(&self) {
        if !self.is_visible.swap(true, Ordering::SeqCst) {
            unsafe {
                let _: () = msg_send![self.window, setAlphaValue: 0.0f64];
                let _: () = msg_send![self.window, orderFrontRegardless];

                let cls = class!(NSAnimationContext);
                let _: () = msg_send![cls, beginGrouping];
                let ctx: id = msg_send![cls, currentContext];
                let _: () = msg_send![ctx, setDuration: 0.15f64];

                let animator: id = msg_send![self.window, animator];
                let _: () = msg_send![animator, setAlphaValue: 1.0f64];

                let _: () = msg_send![cls, endGrouping];
            }
        }
    }

    pub fn hide(&self) {
        if self.is_visible.swap(false, Ordering::SeqCst) {
            unsafe {
                let cls = class!(NSAnimationContext);
                let _: () = msg_send![cls, beginGrouping];
                let ctx: id = msg_send![cls, currentContext];
                let _: () = msg_send![ctx, setDuration: 0.15f64];

                let animator: id = msg_send![self.window, animator];
                let _: () = msg_send![animator, setAlphaValue: 0.0f64];

                let _: () = msg_send![cls, endGrouping];
            }
        }
    }

    pub fn set_color_recording(&self) {
        unsafe {
            let content_view: id = self.window.contentView();
            let layer: id = msg_send![content_view, layer];
            
            // Neon Red
            let red_color = NSColor::colorWithRed_green_blue_alpha_(nil, 1.0, 0.3, 0.3, 1.0);
            let cg_color: id = msg_send![red_color, CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg_color];
            
            // Glow
            let _: () = msg_send![layer, setShadowColor: cg_color];
        }
    }

    pub fn set_color_processing(&self) {
        unsafe {
            let content_view: id = self.window.contentView();
            let layer: id = msg_send![content_view, layer];
            
            // Cyan / Electric Blue
            let blue_color = NSColor::colorWithRed_green_blue_alpha_(nil, 0.0, 0.8, 1.0, 1.0);
            let cg_color: id = msg_send![blue_color, CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg_color];
            
            // Glow
            let _: () = msg_send![layer, setShadowColor: cg_color];
        }
    }

    /// Set indicator to orange/amber color (for errors)
    pub fn set_color_error(&self) {
        unsafe {
            let content_view: id = self.window.contentView();
            let layer: id = msg_send![content_view, layer];
            
            // Orange / Amber
            let orange_color = NSColor::colorWithRed_green_blue_alpha_(nil, 1.0, 0.6, 0.0, 1.0);
            let cg_color: id = msg_send![orange_color, CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg_color];
            
            // Glow
            let _: () = msg_send![layer, setShadowColor: cg_color];
        }
    }

    /// Flash orange briefly to indicate an error, then hide.
    /// Shows error color at full opacity, then immediately starts fade-out.
    pub fn flash_error(&self) {
        unsafe {
            // Make sure we're visible at full opacity with error color
            self.is_visible.store(true, Ordering::SeqCst);
            let _: () = msg_send![self.window, setAlphaValue: 1.0f64];
            let _: () = msg_send![self.window, orderFrontRegardless];
        }
        self.set_color_error();
        
        // Immediately start fade-out (longer duration for flash effect)
        self.is_visible.store(false, Ordering::SeqCst);
        unsafe {
            let cls = class!(NSAnimationContext);
            let _: () = msg_send![cls, beginGrouping];
            let ctx: id = msg_send![cls, currentContext];
            let _: () = msg_send![ctx, setDuration: 0.4f64]; // Longer fade for flash effect

            let animator: id = msg_send![self.window, animator];
            let _: () = msg_send![animator, setAlphaValue: 0.0f64];

            let _: () = msg_send![cls, endGrouping];
        }
    }
}

impl Drop for RecordingIndicator {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.window, close];
        }
    }
}

unsafe impl Send for RecordingIndicator {}
unsafe impl Sync for RecordingIndicator {}
