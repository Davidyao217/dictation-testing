use crate::events::AppEvent;
use muda::{Menu, MenuItem, PredefinedMenuItem};
use tao::event_loop::EventLoopProxy;

pub struct TrayIcon {
    _menu: Menu,
}

impl TrayIcon {
    pub fn new(proxy: EventLoopProxy<AppEvent>) -> anyhow::Result<Self> {
        let menu = Menu::new();
        
        let quit_item = MenuItem::new("Quit Dictation", true, None);
        let quit_id = quit_item.id().clone();
        
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        let proxy_clone = proxy.clone();
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            if event.id == quit_id {
                let _ = proxy_clone.send_event(AppEvent::Quit);
            }
        }));

        log::info!("Tray menu created");

        Ok(Self { _menu: menu })
    }
}
