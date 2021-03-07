use anyhow::Result;
use winreg::{
	RegKey,
	enums::HKEY_CURRENT_USER,
};
use winapi::um::winnt::KEY_ALL_ACCESS;

fn build_value() -> Result<String> {
	let exe = std::env::current_exe()?;
	Ok(format!("\"{}\" --silent", exe.display()))
}

fn registry_key() -> &'static str {
	r#"SOFTWARE\Microsoft\Windows\CurrentVersion\Run"#
}

fn open_key() -> Result<RegKey> {
	let hklm = RegKey::predef(HKEY_CURRENT_USER);
	let key = hklm.open_subkey_with_flags(registry_key(), KEY_ALL_ACCESS)?;
	Ok(key)
}

fn value_name() -> &'static str {
	"Roboto's_Soundboard"
}

pub fn is_enabled() -> Result<bool> {
	let key = open_key()?;
	let value: Option<String> = key.get_value(value_name()).map_or_else(|_| None, Some);

	match value {
		Some(value) => Ok(value == build_value()?),
		None => Ok(false),
	}
}

pub fn enable() -> Result<()> {
	let key = open_key()?;
	let value = build_value()?;
	key.set_value(value_name(), &value)?;

	Ok(())
}

pub fn disable() -> Result<()> {
	let key = open_key()?;
	if key.enum_values().map(|x| x.unwrap()).map(|(n, _v)| n).any(|name| name == value_name()) {
		key.delete_value(value_name())?;
	}

	Ok(())
}
