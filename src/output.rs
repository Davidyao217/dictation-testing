use anyhow::Result;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

use crate::config::OutputMode;

pub struct OutputHandler {
    clipboard: Clipboard,
    enigo: Enigo,
    mode: OutputMode,
}

impl OutputHandler {
    pub fn new(mode: OutputMode) -> Result<Self> {
        let clipboard = Clipboard::new()?;
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { clipboard, enigo, mode })
    }

    pub fn output_text(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            log::warn!("No text to output");
            return Ok(());
        }

        match self.mode {
            OutputMode::Clipboard => self.paste_text(text),
            OutputMode::Keystroke => self.type_text(text),
        }
    }

    fn paste_text(&mut self, text: &str) -> Result<()> {
        log::info!("Pasting text via clipboard: {}", text);

        self.clipboard.set_text(text)?;
        thread::sleep(Duration::from_millis(50));

        self.enigo.key(Key::Meta, Direction::Press)?;
        self.enigo.key(Key::Unicode('v'), Direction::Click)?;
        self.enigo.key(Key::Meta, Direction::Release)?;

        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<()> {
        log::info!("Typing text via keystrokes: {}", text);
        
        for c in text.chars() {
            self.enigo.key(Key::Unicode(c), Direction::Click)?;
            thread::sleep(Duration::from_millis(5));
        }

        Ok(())
    }
}
