use crate::audio;
use crate::config::Sound;

use std::{
	thread,
	time::Duration,
	net::SocketAddr,
	collections::HashMap,
	path::{ PathBuf, Path },
	sync::{
		Arc, Mutex,
		atomic::{ Ordering, AtomicBool },
	},
};

use anyhow::Result;
use serde_json::json;
use rodio::DeviceTrait;
use tokio::net::{ TcpListener, TcpStream };
use tokio_tungstenite::tungstenite::Message;
use futures_channel::mpsc::{ unbounded, UnboundedSender };
use futures_util::{ future, pin_mut, stream::TryStreamExt, StreamExt };

struct Client {
	pub client_type: ClientType,
	pub addr: SocketAddr,
	pub tx: Tx,
}

#[derive(PartialEq, Eq)]
enum ClientType {
	Unknown,
	SettingsWindow,
	SoundboardWindow,
	BrowserSettingsWindow,
	BrowserSoundboardWindow,
}

impl ClientType {
	pub fn parse(t: &str) -> Self {
		if t == "settings" { Self::SettingsWindow }
		else if t == "soundboard" { Self::SoundboardWindow }
		else if t == "browserSettings" { Self::BrowserSettingsWindow }
		else if t == "browserSoundboard" { Self::BrowserSoundboardWindow }
		else { Self::Unknown }
	}
}

type Tx = UnboundedSender<Message>;
type PeerMap = HashMap<SocketAddr, Client>;

#[derive(Default)]
pub struct WebSocketServer {
	app: Option<Arc<crate::App>>,
	peers: PeerMap,
	thread_handle: Option<std::thread::JoinHandle<()>>,
	cancellation_token: Option<tokio_util::sync::CancellationToken>,
}

impl WebSocketServer {
	async fn handle_connection(instance: Arc<Mutex<Self>>, raw_stream: TcpStream, addr: SocketAddr) {
		log::debug!("Incoming TCP connection from: {}", addr);

		let ws_stream = tokio_tungstenite::accept_async(raw_stream)
			.await
			.expect("Error during the websocket handshake occurred");
		log::debug!("WebSocket connection established: {}", addr);

		// Insert the write part of this peer to the peer map.
		let (tx, rx) = unbounded();
		{
			let client = Client {
				client_type: ClientType::Unknown,
				addr,
				tx,
			};
			let mut instance = instance.lock().unwrap();
			instance.send_theme(&client, instance.get_current_theme_name()).unwrap();
			instance.peers.insert(addr, client);
		}

		let (outgoing, incoming) = ws_stream.split();

		let incoming_messages = incoming.try_for_each(|msg| {
			if let Ok(msg) = msg.to_text() {
				log::debug!("Received a message from {}: {}", addr, msg);

				if let Ok(data) = serde_json::from_str::<serde_json::Value>(msg) {
					let msg_type: String = serde_json::from_value(data["message"].clone()).unwrap();

					if msg_type == "clientType" {
						let mut instance = instance.lock().unwrap();
						let mut client = instance.peers.get_mut(&addr).unwrap();
						let client_type: String = serde_json::from_value(data["clientType"].clone()).unwrap();
						client.client_type = ClientType::parse(&client_type);
					}

					let instance = instance.lock().unwrap();
					let client = instance.peers.get(&addr).unwrap();
					let res = match msg_type.as_ref() {
						"sounds" => instance.sounds(&client),
						"playSound" => instance.play_sound(&data),
						"outputDevices" => instance.output_devices(&client),
						"config" => instance.config(&client),
						"setOutputDevices" => instance.set_output_devices(&data),
						"setCopySound" => instance.set_copy_sound(&data),
						"deleteSound" => instance.delete_sound(&client, &data),
						"editSound" => instance.edit_sound(&client, &data),
						"addSounds" => instance.add_sounds(&data),
						"addSoundsDialog" => instance.add_sounds_dialog(&client),
						"browseSoundDialog" => instance.browse_sound_dialog(&client, &data),
						"closeSoundboard" => instance.close_soundboard(),
						"isUpdateAvailable" => instance.is_update_available(&client),
						"performUpdate" => instance.perform_update(),
						"isAutostartEnabled" => instance.is_autostart_enabled(&client),
						"setAutostart" => instance.set_autostart(&data),
						"currentVersion" => instance.current_version(&client),
						"clientType" => Ok(()),
						_ => instance.error(&client, format!("Invalid message type {}", msg_type)),
					};
					if let Err(err) = res {
						log::warn!("Error: {}", err);
					}
				};
			}

			future::ok(())
		});

		let receive_from_others = rx.map(Ok).forward(outgoing);

		pin_mut!(incoming_messages, receive_from_others);
		future::select(incoming_messages, receive_from_others).await;

		log::debug!("{} disconnected", &addr);
		instance.lock().unwrap().peers.remove(&addr);
	}

	#[tokio::main]
	async fn start_inner(instance: Arc<Mutex<Self>>, server_started: Arc<AtomicBool>) {
		let address = "127.0.0.1:29816";

		let listener = crate::utils::retry(|| -> Result<std::net::TcpListener, std::io::Error> {
			let listener = std::net::TcpListener::bind(&address)?;
			listener.set_nonblocking(true)?;
			Ok(listener)
		}, 5, Duration::from_millis(1_000)).expect("Could not bind TcpListener");
		let listener = TcpListener::from_std(listener).expect("Could not create tokio::net::TcpListener");

		log::info!("WebSocket server listening on {}.", address);
		server_started.store(true, Ordering::SeqCst);

		let token = tokio_util::sync::CancellationToken::new();
		{
			let mut instance = instance.lock().unwrap();
			instance.cancellation_token = Some(token.clone());
		}

		let join_handle = tokio::spawn(async move {
			tokio::select! {
				_ = token.cancelled() => {
					// The token was cancelled
				}
				_ = tokio::spawn(async move {
					// Spawn the handling of each connection in a separate task.
					while let Ok((stream, addr)) = listener.accept().await {
						let instance = instance.clone();
						tokio::spawn(Self::handle_connection(instance, stream, addr));
					}
				}) => { }
			}
		});

		join_handle.await.unwrap();
	}

	pub fn start(instance: Arc<Mutex<Self>>, app: Arc<crate::App>, server_started: Arc<AtomicBool>) -> Result<(), std::io::Error> {
		instance.lock().unwrap().app = Some(app);
		let clone = instance.clone();
		let handle = std::thread::spawn(move || Self::start_inner(clone, server_started));
		instance.lock().unwrap().thread_handle = Some(handle);

		Ok(())
	}

	pub fn stop(&mut self) {
		log::info!("Stopping WebSocket server...");

		if let Some(token) = &self.cancellation_token {
			token.cancel();
			if let Some(handle) = self.thread_handle.take() {
				let _ = handle.join();
			}
		}
	}

	pub fn set_window_theme(&self, theme: winit::window::Theme) -> Result<()> {
		for client in self.peers.values() {
			if client.client_type == ClientType::SettingsWindow || client.client_type == ClientType::SoundboardWindow {
				self.send_theme(client, self.get_theme_name(theme))?;
			}
		}

		Ok(())
	}

	pub fn settings_add_sounds_dialog_callback(&self, files: &[PathBuf]) {
		let paths = files.iter().map(|f| f.to_string_lossy().into_owned()).collect();
		if let Err(err) = self.do_add_sounds(paths) {
			log::error!("Error in settings_add_sounds_dialog_callback: {}", err);
		}
	}

	pub fn settings_browse_sound_dialog_callback(&self, request_id: u32, new_path: &PathBuf) {
		let msg = json!({
			"message": "requestCallback",
			"requestId": request_id,
			"newPath": new_path.to_string_lossy().into_owned(),
		}).to_string().into();

		let _ = self.broadcast(None, msg, Some(vec![
			ClientType::SettingsWindow,
			ClientType::SoundboardWindow,
			ClientType::BrowserSettingsWindow,
			ClientType::BrowserSoundboardWindow,
		]));
	}

	pub fn send_soundboard_window_opened_event(&self) {
		let msg = json!({
			"message": "soundboardOpened",
		})
		.to_string()
		.into();

		if let Err(err) = self.broadcast(None, msg, Some(vec![ClientType::SoundboardWindow])) {
			log::error!("Error in send_soundboard_window_opened_event: {}", err);
		}
	}

	fn process_sound_path(path: &mut String, app_dir: Option<&PathBuf>) -> Result<()> {
		let app_dir = match app_dir {
			Some(dir) => dir.clone(),
			None => crate::App::get_working_directory()?,
		};

		// Use relative path if the file is in a subfolder of the app dir
		let path_buf = PathBuf::from(&path);
		if crate::utils::path_is_child(&app_dir, &path_buf)? {
			if let Ok(stripped) = path_buf.strip_prefix(app_dir) {
				*path = stripped.to_string_lossy().into_owned();
			}
		}

		Ok(())
	}

	fn sounds(&self, client: &Client) -> Result<()> {
		let config = self.app.as_ref().unwrap().config.lock().unwrap();
		client.tx.unbounded_send(
			json!({
				"message": "sounds",
				"sounds": &config.sounds,
			})
			.to_string()
			.into()
		)?;

		Ok(())
	}

	fn play_sound(&self, data: &serde_json::Value) -> Result<()> {
		let config = self.app.as_ref().unwrap().config.lock().unwrap();
		let sound_path: String = serde_json::from_value(data["sound"].clone())?;
		let sound = config.sounds.iter().find(|sound| sound.path == sound_path);
		if let Some(sound) = sound {
			for dev in config.output_devices.clone() {
				let sound_path = sound_path.clone();
				let volume = config.global_volume * dev.volume * sound.volume;
				thread::spawn(move || {
					match audio::play_sound(sound_path.as_ref(), dev.name.as_ref()) {
						Ok(res) => {
							res.sound.set_volume(volume);
							res.sound.sleep_until_end();
						},
						Err(err) => log::error!("Could not play sound: {}", err),
					}
				});
			}
		}

		Ok(())
	}

	fn output_devices(&self, client: &Client) -> Result<()> {
		let devices: Vec<String> = audio::get_output_devices()?
			.iter()
			.map(|dev| dev.name().unwrap())
			.collect();

		client.tx.unbounded_send(
			json!({
				"message": "outputDevices",
				"devices": devices,
			})
			.to_string()
			.into()
		)?;

		Ok(())
	}

	fn config(&self, client: &Client) -> Result<()> {
		let config = &self.app.as_ref().unwrap().config;

		client.tx.unbounded_send(
			json!({
				"message": "config",
				"config": &config,
			})
			.to_string()
			.into()
		)?;

		Ok(())
	}

	fn set_output_devices(&self, data: &serde_json::Value) -> Result<()> {
		let mut config = self.app.as_ref().unwrap().config.lock().unwrap();
		config.output_devices = serde_json::from_value(data["devices"].clone())?;
		config.save();

		let msg = json!({
			"message": "configOutputDevices",
			"config": &config.output_devices,
		})
		.to_string()
		.into();
		self.broadcast(None, msg, Some(vec![ClientType::SettingsWindow, ClientType::BrowserSettingsWindow]))?;

		Ok(())
	}

	fn set_copy_sound(&self, data: &serde_json::Value) -> Result<()> {
		let enabled: bool = serde_json::from_value(data["enabled"].clone())?;
		let mut config = self.app.as_ref().unwrap().config.lock().unwrap();
		config.sounds_config.copy_file = enabled;
		config.save();

		let msg = json!({
			"message": "configSoundsConfig",
			"config": &config.sounds_config,
		})
		.to_string()
		.into();
		self.broadcast(None, msg, Some(vec![ClientType::SettingsWindow, ClientType::BrowserSettingsWindow]))?;

		Ok(())
	}

	fn delete_sound(&self, client: &Client, data: &serde_json::Value) -> Result<()> {
		let sound_path: String = serde_json::from_value(data["sound"].clone())?;
		let mut config = self.app.as_ref().unwrap().config.lock().unwrap();
		config.sounds.retain(|sound| sound.path != sound_path);
		config.save();

		let msg = json!({
			"message": "removedSound",
			"path": sound_path,
		}).to_string().into();
		self.broadcast(Some(client), msg, Some(vec![
			ClientType::SettingsWindow,
			ClientType::SoundboardWindow,
			ClientType::BrowserSettingsWindow,
			ClientType::BrowserSoundboardWindow,
		]))?;

		Ok(())
	}

	fn edit_sound(&self, client: &Client, data: &serde_json::Value) -> Result<()> {
		let mut sound_edited: Sound = serde_json::from_value(data["sound"].clone())?;
		let receive_update: Option<bool> = serde_json::from_value(data["receiveUpdate"].clone())?;
		let prev_path: Result<String, serde_json::Error> = serde_json::from_value(data["previousPath"].clone());
		let prev_path = match prev_path {
			Ok(prev_path) => prev_path, // Use the previous path if any (which means the path has been modified)
			Err(_) => sound_edited.path.clone(), // If the previous path is not specified, use the one in the sound struct
		};

		Self::process_sound_path(&mut sound_edited.path, None)?;

		let mut config = self.app.as_ref().unwrap().config.lock().unwrap();
		let sound = config.sounds.iter_mut().find(|sound| sound.path == prev_path);
		if let Some(sound) = sound {
			let msg = json!({
				"message": "modifiedSound",
				"previousPath": prev_path,
				"sound": sound_edited,
			}).to_string().into();

			*sound = sound_edited;
			config.save();

			let except = match receive_update {
				Some(true) => None,
				_ => Some(client),
			};
			self.broadcast(except, msg, Some(vec![
				ClientType::SettingsWindow,
				ClientType::SoundboardWindow,
				ClientType::BrowserSettingsWindow,
				ClientType::BrowserSoundboardWindow,
			]))?;
		}

		Ok(())
	}

	fn add_sounds(&self, data: &serde_json::Value) -> Result<()> {
		self.do_add_sounds(serde_json::from_value(data["files"].clone())?)?;
		Ok(())
	}

	fn do_add_sounds(&self, mut paths: Vec<String>) -> Result<()> {
		let mut config = self.app.as_ref().unwrap().config.lock().unwrap();
		let app_dir = crate::App::get_working_directory()?;

		if config.sounds_config.copy_file {
			let mut sounds_dir = app_dir.clone();
			sounds_dir.push("sounds");
			if !sounds_dir.exists() {
				std::fs::create_dir(&sounds_dir)?;
			}

			for path in paths.iter_mut() {
				let path_buf = PathBuf::from(&path);
				if !crate::utils::path_is_child(&sounds_dir, &path_buf)? {
					let mut target_path = sounds_dir.clone();
					target_path.push(path_buf.file_name().unwrap());
					std::fs::copy(&path, &target_path)?;
					*path = target_path.to_string_lossy().into_owned();
				}
			}
		}

		for path in paths.iter_mut() {
			Self::process_sound_path(path, Some(&app_dir))?;
		}

		paths.retain(|path| !config.sounds.iter().any(|sound| sound.path == *path));
		let sounds: Vec<Sound> = paths
			.iter()
			.map(|path| Sound {
				path: path.to_string(),
				name: Path::new(&path).file_stem().unwrap().to_os_string().into_string().unwrap(),
				volume: 1.0,
				image: None,
			})
			.collect();
		let msg = json!({
			"message": "addedSounds",
			"sounds": &sounds,
		}).to_string().into();
		config.sounds.extend(sounds);
		config.save();

		self.broadcast(None, msg, Some(vec![
			ClientType::SettingsWindow,
			ClientType::SoundboardWindow,
			ClientType::BrowserSettingsWindow,
			ClientType::BrowserSoundboardWindow,
		]))?;

		Ok(())
	}

	fn add_sounds_dialog(&self, client: &Client) -> Result<()> {
		if let ClientType::SettingsWindow = client.client_type {
			self.app
				.as_ref().unwrap()
				.window_manager.lock().unwrap()
				.settings_window_add_sounds_dialog()?;
		}

		Ok(())
	}

	fn browse_sound_dialog(&self, client: &Client, data: &serde_json::Value) -> Result<()> {
		let request_id: u32 = serde_json::from_value(data["requestId"].clone())?;
		if let ClientType::SettingsWindow = client.client_type {
			self.app
				.as_ref().unwrap()
				.window_manager.lock().unwrap()
				.settings_window_browse_sound_dialog(request_id)?;
		}

		Ok(())
	}

	fn get_theme_name(&self, theme: winit::window::Theme) -> &'static str {
		match theme {
			winit::window::Theme::Light => "light",
			winit::window::Theme::Dark => "dark",
		}
	}

	fn get_current_theme_name(&self) -> &'static str {
		let theme = self.app
			.as_ref().unwrap()
			.window_manager.lock().unwrap()
			.get_theme();

		match theme {
			Some(theme) => self.get_theme_name(theme),
			None => "dark",
		}
	}

	fn send_theme(&self, client: &Client, theme_name: &str) -> Result<()> {
		client.tx.unbounded_send(
			json!({
				"message": "theme",
				"theme": theme_name,
			})
			.to_string()
			.into()
		)?;

		Ok(())
	}

	fn close_soundboard(&self) -> Result<()> {
		self.app
			.as_ref().unwrap()
			.window_manager.lock().unwrap()
			.close_soundboard_window()?;

		Ok(())
	}

	fn is_update_available(&self, client: &Client) -> Result<()> {
		let available = self.app
			.as_ref().unwrap()
			.update_available.is_some();

		if available {
			client.tx.unbounded_send(
				json!({
					"message": "updateAvailable",
				})
				.to_string()
				.into()
			)?;
		}

		Ok(())
	}

	fn perform_update(&self) -> Result<()> {
		let app = self.app.as_ref().unwrap();
		let download_url = match app.update_available.as_ref() {
			Some(url) => url,
			None => return Ok(()),
		};

		crate::updater::download_update(&download_url, || {
			app.window_manager.lock().unwrap().quit_app().unwrap();
		})?;

		Ok(())
	}

	fn is_autostart_enabled(&self, client: &Client) -> Result<()> {
		let enabled = crate::autostart::is_enabled()?;

		client.tx.unbounded_send(
			json!({
				"message": "autostart",
				"enabled": enabled,
			})
			.to_string()
			.into()
		)?;

		Ok(())
	}

	fn set_autostart(&self, data: &serde_json::Value) -> Result<()> {
		let enabled: bool = serde_json::from_value(data["enabled"].clone())?;

		if enabled {
			crate::autostart::enable()?;
		} else {
			crate::autostart::disable()?;
		}
		for client in self.peers.values() {
			if client.client_type == ClientType::SettingsWindow || client.client_type == ClientType::BrowserSettingsWindow {
				self.is_autostart_enabled(client)?;
			}
		}

		Ok(())
	}

	fn current_version(&self, client: &Client) -> Result<()> {
		client.tx.unbounded_send(
			json!({
				"message": "currentVersion",
				"version": env!("CARGO_PKG_VERSION"),
			})
			.to_string()
			.into()
		)?;

		Ok(())
	}

	fn error(&self, client: &Client, error: String) -> Result<()> {
		client.tx.unbounded_send(
			json!({
				"message": "error",
				"error": error,
			})
			.to_string()
			.into()
		)?;

		Ok(())
	}

	fn broadcast<I>(&self, except: Option<&Client>, msg: Message, target_clients: Option<I>) -> Result<()>
	where I: IntoIterator<Item = ClientType> {
		let filter = |c: &Client| -> bool {
			match except {
				Some(except) => c.addr != except.addr,
				None => true,
			}
		};

		match target_clients {
			Some(target_clients) => {
				for target in target_clients.into_iter() {
					for client in self.peers.values().filter(|c| filter(c)) {
						if target == client.client_type {
							client.tx.unbounded_send(msg.clone())?;
						}
					}
				}
			},
			None => {
				for client in self.peers.values().filter(|c| filter(c)) {
					client.tx.unbounded_send(msg.clone())?;
				}
			},
		};

		Ok(())
	}
}
