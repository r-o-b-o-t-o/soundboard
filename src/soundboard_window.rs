use std::mem;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use winit::{
	error::OsError,
	window::Window,
};

use winapi::{
	um::winuser::{ self, GetClientRect },
	shared::windef::{ HWND, RECT },
};
use winit::{
	window::WindowBuilder,
	platform::windows::WindowExtWindows,
	event_loop::{ ControlFlow, EventLoop },
	dpi::{ PhysicalSize, PhysicalPosition, Size },
};
use webview2::Controller;
use once_cell::unsync::OnceCell;

use crate::window_manager::{ CustomEvent, WindowWrapper, WindowEventHandler };

pub struct SoundboardWindow {
	app: Arc<crate::App>,
	window_wrapper: WindowWrapper,
}

unsafe impl Send for SoundboardWindow {}

impl SoundboardWindow {
	pub fn new(app: Arc<crate::App>, event_loop: &EventLoop<CustomEvent>) -> Result<Self> {
		let window = Self::create_window(event_loop)?;
		let controller = Self::build_webview(&window);

		Ok(Self {
			app,
			window_wrapper: WindowWrapper::new(window, controller),
		})
	}

	fn create_window(event_loop: &EventLoop<CustomEvent>) -> Result<Window, OsError> {
		let (screen_w, screen_h) = crate::utils::get_monitor_size();
		let w = (screen_w as f32 * 0.5) as i32;
		let h = (screen_h as f32 * 0.7) as i32;

		let win = WindowBuilder::new()
			.with_title("Soundboard")
			.with_inner_size(Size::Logical((w, h).into()))
			.with_decorations(false)
			.with_visible(false)
			.with_window_icon(Some(Self::load_icon()))
			.build(event_loop)?;
			win.set_outer_position(PhysicalPosition::new((screen_w - w) / 2, (screen_h - h) / 2));
		Ok(win)
	}

	fn build_webview(window: &Window) -> Rc<OnceCell<Controller>> {
		let controller: Rc<OnceCell<Controller>> = Rc::new(OnceCell::new());

		let create_result = {
			let controller_clone = controller.clone();
			let hwnd = window.hwnd() as HWND;

			webview2::Environment::builder().build(move |env| {
				env.expect("env")
					.create_controller(hwnd, move |controller| {
						let controller = controller.expect("create host");
						let w = controller.get_webview().expect("get_webview");

						let _ = w.get_settings().map(|settings| {
							let _ = settings.put_is_status_bar_enabled(false);
							let _ = settings.put_are_default_context_menus_enabled(false);
							let _ = settings.put_is_zoom_control_enabled(false);
						});

						unsafe {
							let mut rect = mem::zeroed();
							GetClientRect(hwnd, &mut rect);
							controller.put_bounds(rect).expect("put_bounds");
						}

						w.navigate("http://127.0.0.1:29815/soundboard.html").expect("navigate");

						controller_clone.set(controller).unwrap();
						Ok(())
					})
			})
		};
		if let Err(e) = create_result {
			log::error!(
				"Failed to create webview environment: {}. Is the new edge browser installed?",
				e
			);
		}

		controller
	}

	fn load_icon() -> winit::window::Icon {
		let (icon_rgba, icon_width, icon_height) = {
			let mut path = crate::App::get_web_resources_directory().expect("Could not get web resources directory");
			path.push("favicon.ico");
			let image = image::open(path)
				.expect("Failed to open icon")
				.into_rgba8();
			let (width, height) = image.dimensions();
			let rgba = image.into_raw();
			(rgba, width, height)
		};
		winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
	}
}

impl WindowEventHandler for SoundboardWindow {
	fn open(&self) {
		self.window_wrapper.window.set_visible(true);
		if let Some(webview_host) = self.window_wrapper.controller.get() {
			webview_host.put_is_visible(true).expect("Could not show SoundboardWindow");
		}

		unsafe {
			let hwnd = self.window_wrapper.window.hwnd() as HWND;

			winuser::SetForegroundWindow(hwnd);
			winuser::SetFocus(hwnd);

			if let Some(webview_host) = self.window_wrapper.controller.get() {
				if let Err(err) = webview_host.move_focus(webview2::MoveFocusReason::Next) {
					log::error!("move_focus error: {}", err);
				}
			}
		}
	}

	fn on_closed(&self, _control_flow: &mut ControlFlow) {
		self.window_wrapper.window.set_visible(false);
		if let Some(webview_host) = self.window_wrapper.controller.get() {
			webview_host.put_is_visible(false).expect("Could not hide SoundboardWindow");
		}
	}

	fn on_moved(&self) {
		if let Some(webview_host) = self.window_wrapper.controller.get() {
			let _ = webview_host.notify_parent_window_position_changed();
		}
	}

	fn on_resized(&self, new_size: PhysicalSize<u32>) {
		if let Some(webview_host) = self.window_wrapper.controller.get() {
			let r = RECT {
				left: 0,
				top: 0,
				right: new_size.width as i32,
				bottom: new_size.height as i32,
			};
			webview_host.put_bounds(r).expect("put_bounds");
		}
	}

	fn on_theme_changed(&self, new_theme: winit::window::Theme) {
		let _ = self.app.websocket_server.lock().unwrap().set_window_theme(new_theme);
	}

	fn window_wrapper(&self) -> &WindowWrapper {
		&self.window_wrapper
	}
}
