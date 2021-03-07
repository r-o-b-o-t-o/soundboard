use std::rc::Rc;
use std::sync::{ Arc, Mutex };

use anyhow::Result;
use winit::{
	dpi::PhysicalSize,
	platform::windows::WindowExtWindows,
	platform::run_return::EventLoopExtRunReturn,
	window::{ Window, WindowId },
	event::{ Event, WindowEvent },
	event_loop::{ ControlFlow, EventLoop, EventLoopProxy },
};
use webview2::Controller;
use once_cell::unsync::OnceCell;
use trayicon::{ MenuBuilder, TrayIconBuilder };

use crate::settings_window::SettingsWindow;
use crate::soundboard_window::SoundboardWindow;

pub struct WindowWrapper {
	pub window: Window,
	pub controller: Rc<OnceCell<Controller>>,
}

impl WindowWrapper {
	pub fn new(window: Window, controller: Rc<OnceCell<Controller>>) -> Self {
		Self {
			window,
			controller,
		}
	}
}

pub trait WindowEventHandler {
	fn open(&self) {}
	fn on_closed(&self, _control_flow: &mut ControlFlow) {}
	fn on_moved(&self) {}
	fn on_resized(&self, _new_size: PhysicalSize<u32>) {}
	fn on_theme_changed(&self, _new_theme: winit::window::Theme) {}
	fn window_wrapper(&self) -> &WindowWrapper;
}

#[derive(Default)]
pub struct WindowManager {
	app: Option<Arc<crate::App>>,
	event_loop_proxy: Option<EventLoopProxy<CustomEvent>>,
	settings_window: Option<SettingsWindow>,
	soundboard_window: Option<SoundboardWindow>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CustomEvent {
	QuitApp,
	OpenSettingsWindow,
	OpenSoundboardWindow,
	CloseSoundboardWindow,
	SettingsAddSoundsDialog,
	SettingsBrowseSoundDialog(u32),
}

impl WindowManager {
	pub fn run(instance: Arc<Mutex<Self>>, app: Arc<crate::App>) -> Result<()> {
		let mut event_loop = EventLoop::<CustomEvent>::with_user_event();
		let event_loop_proxy = event_loop.create_proxy();

		let _tray_icon = TrayIconBuilder::new()
			.sender_winit(event_loop_proxy.clone())
			.icon_from_buffer(include_bytes!("../soundboard-ui/static/favicon.ico"))
			.tooltip("Soundboard")
			.on_click(CustomEvent::OpenSoundboardWindow)
			.menu(MenuBuilder::new()
				.item("Settings", CustomEvent::OpenSettingsWindow)
				.item("Soundboard", CustomEvent::OpenSoundboardWindow)
				.separator()
				.item("&Quit", CustomEvent::QuitApp)
			)
			.build()
			.unwrap();

		{
			let mut instance = instance.lock().unwrap();
			instance.app = Some(app.clone());
			instance.event_loop_proxy = Some(event_loop_proxy);
			instance.settings_window = Some(SettingsWindow::new(app.clone(), &event_loop)?);
			instance.soundboard_window = Some(SoundboardWindow::new(app, &event_loop)?);
		}

		event_loop.run_return(move |event, _event_loop, control_flow| {
			*control_flow = ControlFlow::Wait;

			match event {
				Event::UserEvent(event) => {
					match event {
						CustomEvent::QuitApp => {
							*control_flow = ControlFlow::Exit;
						},
						CustomEvent::OpenSettingsWindow => {
							let instance = instance.lock().unwrap();
							if let Some(win) = instance.settings_window.as_ref() {
								win.open();
							}
						},
						CustomEvent::OpenSoundboardWindow => {
							let instance = instance.lock().unwrap();
							if let Some(win) = instance.soundboard_window.as_ref() {
								win.open();

								instance.app
									.as_ref().unwrap()
									.websocket_server.lock().unwrap()
									.send_soundboard_window_opened_event();
							}
						},
						CustomEvent::CloseSoundboardWindow => {
							let instance = instance.lock().unwrap();
							if let Some(win) = instance.soundboard_window.as_ref() {
								win.on_closed(control_flow);
							}
						},
						CustomEvent::SettingsAddSoundsDialog => {
							let instance = instance.lock().unwrap();
							if let Some(window) = &instance.settings_window {
								let files = window.open_add_sounds_dialog(true);
								if !files.is_empty() {
									instance.app
										.as_ref().unwrap()
										.websocket_server.lock().unwrap()
										.settings_add_sounds_dialog_callback(&files);
								}
							}
						},
						CustomEvent::SettingsBrowseSoundDialog(request_id) => {
							let instance = instance.lock().unwrap();
							if let Some(window) = &instance.settings_window {
								let files = window.open_add_sounds_dialog(false);
								if !files.is_empty() {
									instance.app
										.as_ref().unwrap()
										.websocket_server.lock().unwrap()
										.settings_browse_sound_dialog_callback(request_id, &files.first().unwrap());
								}
							}
						},
					}
				},
				Event::WindowEvent { event, window_id } => {
					let instance = instance.lock().unwrap();
					if let Some(window) = instance.get_window(&window_id) {
						match event {
							WindowEvent::CloseRequested => {
								window.window_wrapper().window.set_maximized(false);
								window.on_closed(control_flow);
							},
							WindowEvent::Moved(_) => window.on_moved(),
							WindowEvent::Resized(new_size) => window.on_resized(new_size),
							WindowEvent::ThemeChanged(theme) => window.on_theme_changed(theme),
							_ => (),
						}
					}
				},
				_ => (),
			}
		});

		Ok(())
	}

	fn get_window(&self, window_id: &WindowId) -> Option<&dyn WindowEventHandler> {
		if let Some(win) = self.settings_window.as_ref() {
			if win.window_wrapper().window.id() == *window_id {
				return Some(win);
			}
		}
		if let Some(win) = self.soundboard_window.as_ref() {
			if win.window_wrapper().window.id() == *window_id {
				return Some(win);
			}
		}

		None
	}

	pub fn get_theme(&self) -> Option<winit::window::Theme> {
		self.settings_window.as_ref().map(|s_win| s_win.window_wrapper().window.theme())
	}

	pub fn settings_window_add_sounds_dialog(&self) -> Result<()> {
		if let Some(proxy) = &self.event_loop_proxy {
			proxy.send_event(CustomEvent::SettingsAddSoundsDialog)?;
		}

		Ok(())
	}

	pub fn settings_window_browse_sound_dialog(&self, request_id: u32) -> Result<()> {
		if let Some(proxy) = &self.event_loop_proxy {
			proxy.send_event(CustomEvent::SettingsBrowseSoundDialog(request_id))?;
		}

		Ok(())
	}

	pub fn open_soundboard_window(&self) -> Result<()> {
		if let Some(proxy) = &self.event_loop_proxy {
			proxy.send_event(CustomEvent::OpenSoundboardWindow)?;
		}

		Ok(())
	}

	pub fn close_soundboard_window(&self) -> Result<()> {
		if let Some(proxy) = &self.event_loop_proxy {
			proxy.send_event(CustomEvent::CloseSoundboardWindow)?;
		}

		Ok(())
	}

	pub fn quit_app(&self) -> Result<()> {
		if let Some(proxy) = &self.event_loop_proxy {
			proxy.send_event(CustomEvent::QuitApp)?;
		}

		Ok(())
	}
}
