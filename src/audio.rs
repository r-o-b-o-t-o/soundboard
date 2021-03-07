use std::io::BufReader;

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};

pub struct PlaySound {
	pub sound: rodio::Sink,
	_stream: rodio::OutputStream,
	_stream_handle: rodio::OutputStreamHandle,
}

impl PlaySound {
	pub fn new(stream: rodio::OutputStream, stream_handle: rodio::OutputStreamHandle, sink: rodio::Sink) -> Self {
		Self {
			sound: sink,
			_stream: stream,
			_stream_handle: stream_handle,
		}
	}
}

pub fn get_output_devices() -> Result<Vec<cpal::Device>> {
	let mut devices = Vec::new();

	for host_id in cpal::available_hosts() {
		let host = cpal::host_from_id(host_id)?;

		devices.extend(
			host.devices()?
				.filter(|device| device.default_output_config().is_ok()),
		);
	}

	Ok(devices)
}

pub fn play_sound(sound_path: &str, device_name: &str) -> Result<PlaySound> {
	let devices = get_output_devices()?;
	let device = devices
		.iter()
		.find(|device| device.name().unwrap() == device_name)
		.ok_or_else(|| {
			std::io::Error::new(
				std::io::ErrorKind::Other,
				format!("Device {} not found", device_name),
			)
		})?;
	play_sound_on_device(sound_path, device)
}

pub fn play_sound_on_device(sound_path: &str, device: &cpal::Device) -> Result<PlaySound> {
	let (stream, stream_handle) = rodio::OutputStream::try_from_device(device)?;

	let file = std::fs::File::open(sound_path)?;
	let sound = stream_handle
		.play_once(BufReader::new(file))
		.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", err)))?;

	Ok(PlaySound::new(stream, stream_handle, sound))
}
