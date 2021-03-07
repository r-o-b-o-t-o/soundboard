use std::fs::File;
use std::path::PathBuf;
use std::io::BufReader;

use anyhow::Result;
use serde::{ Serialize, Deserialize };

#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SoundImage {
	Url(String),
	File(String),
}

#[serde(default)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Sound {
	pub name: String,
	pub path: String,
	pub volume: f32,
	pub image: Option<SoundImage>,
}

#[serde(default)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OutputDevice {
	pub name: String,
	pub volume: f32,
}

#[serde(default)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SoundsConfig {
	pub copy_file: bool,
}

#[serde(default)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	pub global_volume: f32,
	pub output_devices: Vec<OutputDevice>,
	pub sounds_config: SoundsConfig,
	pub sounds: Vec<Sound>,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			sounds: Vec::new(),
			global_volume: 1.0,
			sounds_config: SoundsConfig::default(),
			output_devices: Vec::new(),
		}
	}
}

impl Config {
	fn config_file_path() -> Result<PathBuf> {
		let mut path = crate::App::get_working_directory()?;
		path.push("config.json");
		Ok(path)
	}

	fn read_from_file() -> Result<Self> {
		let path = Self::config_file_path()?;
		let file = File::open(path)?;
		let reader = BufReader::new(file);
		let cfg = serde_json::from_reader(reader)?;
		Ok(cfg)
	}

	pub fn read() -> Self {
		match Self::read_from_file() {
			Ok(cfg) => cfg,
			Err(err) => {
				log::warn!("Could not read configuration from file (falling back to default configuration): {}", err);
				let cfg = Self::default();
				if !Self::config_file_path().unwrap().exists() {
					cfg.save();
				}
				cfg
			}
		}
	}

	fn save_to_file(&self) -> Result<()> {
		let path = Self::config_file_path()?;
		let buf = Vec::new();
		let formatter = serde_json::ser::PrettyFormatter::with_indent(b"	");
		let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
		self.serialize(&mut ser)?;
		std::fs::write(path, String::from_utf8(ser.into_inner())?)?;
		Ok(())
	}

	pub fn save(&self) {
		if let Err(err) = self.save_to_file() {
			log::error!("Could not save configuration file: {}", err);
		}
	}
}
