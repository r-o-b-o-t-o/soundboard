use std::io::prelude::*;
use std::io::{ Seek, Write };
use std::fs::File;
use std::path::Path;
use std::iter::Iterator;

use anyhow::Result;
use zip::result::ZipError;
use zip::write::FileOptions;
use walkdir::{ DirEntry, WalkDir };

pub fn main() -> Result<()> {
	let src_dir = "package";
	let dst_file = format!("soundboard-v{}-{}-{}.zip", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH);
	zip_package(src_dir, &dst_file, zip::CompressionMethod::Bzip2)?;
	Ok(())
}

fn zip_dir<T>(it: &mut dyn Iterator<Item = DirEntry>, prefix: &str, writer: T, method: zip::CompressionMethod) -> zip::result::ZipResult<()>
where T: Write + Seek {
	let mut zip = zip::ZipWriter::new(writer);
	let options = FileOptions::default()
		.compression_method(method)
		.unix_permissions(0o755);

	let mut buffer = Vec::new();
	for entry in it {
		let path = entry.path();
		let name = path.strip_prefix(Path::new(prefix)).unwrap();

		// Write file or directory explicitly
		// Some unzip tools unzip files with directory paths correctly, some do not!
		if path.is_file() {
			#[allow(deprecated)]
			zip.start_file_from_path(name, options)?;
			let mut f = File::open(path)?;

			f.read_to_end(&mut buffer)?;
			zip.write_all(&*buffer)?;
			buffer.clear();
		} else if name.as_os_str().len() != 0 {
			// Only if not root! Avoids path spec / warning
			// and mapname conversion failed error on unzip
			#[allow(deprecated)]
			zip.add_directory_from_path(name, options)?;
		}
	}
	zip.finish()?;
	Result::Ok(())
}

fn zip_package(src_dir: &str, dst_file: &str, method: zip::CompressionMethod) -> zip::result::ZipResult<()> {
	if !Path::new(src_dir).is_dir() {
		return Err(ZipError::FileNotFound);
	}

	let path = Path::new(dst_file);
	let file = File::create(&path).unwrap();

	let walkdir = WalkDir::new(src_dir.to_string());
	let it = walkdir.into_iter();

	zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, file, method)?;

	Ok(())
}
