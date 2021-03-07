#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Don't open a cmd window when running on Windows in release mode

use std::path::PathBuf;
use std::sync::{
	Arc, Mutex,
	atomic::{ AtomicBool, Ordering },
};

use anyhow::Result;
use argparse::{ ArgumentParser, StoreTrue };

pub mod ws;
pub mod audio;
pub mod utils;
pub mod config;
pub mod updater;
pub mod autostart;
pub mod web_server;
pub mod window_manager;
pub mod settings_window;
pub mod soundboard_window;

use config::Config;

pub struct App {
	pub args: Args,
	pub config: Mutex<Config>,
	pub update_available: Option<String>,
	pub web_server: Arc<Mutex<web_server::WebServer>>,
	pub websocket_server: Arc<Mutex<ws::WebSocketServer>>,
	pub window_manager: Arc<Mutex<window_manager::WindowManager>>,
}

impl App {
	pub fn get_working_directory() -> std::io::Result<PathBuf> {
		if cfg!(debug_assertions) {
			std::env::current_dir()
		} else {
			let mut path = std::env::current_exe()?;
			path.pop();
			Ok(path)
		}
	}

	pub fn get_web_resources_directory() -> std::io::Result<PathBuf> {
		let mut path = Self::get_working_directory()?;
		if cfg!(debug_assertions) {
			path.push("soundboard-ui");
			path.push("dist");
		} else {
			path.push("resources");
		}
		Ok(path)
	}

	fn set_ctrl_c_handler(&self) -> Result<()> {
		let window_manager = self.window_manager.clone();

		ctrlc::set_handler(move || {
			log::info!("Exiting...");
			if let Err(err) = window_manager.lock().unwrap().quit_app() {
				log::error!("WindowManager quit_app error: {}", err);
			}
		})?;

		Ok(())
	}

	fn set_global_shortcut(&self) -> std::thread::JoinHandle<()> {
		let window_manager = self.window_manager.clone();

		std::thread::spawn(move || {
			let mut hk = hotkey::Listener::new();
			hk.register_hotkey(
				hotkey::modifiers::CONTROL | hotkey::modifiers::SHIFT, ' ' as u32,
				move || {
					if let Err(err) = window_manager.lock().unwrap().open_soundboard_window() {
						log::error!("WindowManager open_soundboard_window error: {}", err);
					}
				},
			).unwrap();

			hk.listen();
		})
	}

	fn check_for_updates(&mut self) -> Result<()> {
		if let Some(download_url) = updater::is_update_available()? {
			log::info!("An update is available!");
			self.update_available = Some(download_url);
		}
		Ok(())
	}
}

#[derive(Default)]
pub struct Args {
	silent: bool,
}

impl Args {
	pub fn parse() -> Self {
		let mut args = Self::default();

		{
			let mut ap = ArgumentParser::new();
			ap.refer(&mut args.silent)
				.add_option(&["-s", "--silent"], StoreTrue, "Start with the settings window hidden");
			let _ = ap.parse_args();
		}

		updater::check_args();

		args
	}
}

fn setup_logging() -> Result<()> {
	let mut log_file = App::get_working_directory()?;
	log_file.push("soundboard.log");

	Ok(fern::Dispatch::new()
		.format(|out, message, record| {
			out.finish(format_args!(
				"{}[{}][{}] {}",
				chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
				record.target(),
				record.level(),
				message
			))
		})
		.level(log::LevelFilter::Info)
		.level_for("actix_server", log::LevelFilter::Warn)
		.level_for("ureq", log::LevelFilter::Warn)
		.chain(std::io::stdout())
		.chain(std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.open(log_file)?)
		.apply()?)
}

#[actix_web::main]
async fn main() -> Result<()> {
	utils::setup_panic_hook();

	if let Err(err) = setup_logging() {
		eprintln!("Could not setup logging: {}", err);
	}
	log::info!("Version {}", env!("CARGO_PKG_VERSION"));
	log::info!("PID: {}", std::process::id());

	let mut app = App {
		args: Args::parse(),
		update_available: None,
		config: Mutex::new(Config::read()),
		web_server: Arc::new(Mutex::new(web_server::WebServer::default())),
		websocket_server: Arc::new(Mutex::new(ws::WebSocketServer::default())),
		window_manager: Arc::new(Mutex::new(window_manager::WindowManager::default())),
	};
	if let Err(err) = app.check_for_updates() {
		log::warn!("Update check error: {}", err);
	}
	app.set_ctrl_c_handler()?;
	app.set_global_shortcut();
	let app = Arc::new(app);

	// Start websocket server
	let server_started = Arc::new(AtomicBool::new(false));
	let server_started_clone = server_started.clone();
	ws::WebSocketServer::start(app.websocket_server.clone(), app.clone(), server_started_clone)?;
	while !server_started.load(Ordering::SeqCst) {
		std::thread::sleep(std::time::Duration::from_millis(100));
	}

	// Start serving web assets
	app.web_server.lock().unwrap().start()?;

	// Create windows
	let win_mngr = app.window_manager.clone();
	window_manager::WindowManager::run(win_mngr, app.clone())?;

	app.websocket_server.lock().unwrap().stop();
	app.web_server.lock().unwrap().stop().await;

	Ok(())
}
