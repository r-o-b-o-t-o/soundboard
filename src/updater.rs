use std::{
	thread,
	io::Read,
	path::PathBuf,
	cmp::Ordering,
	time::Duration,
	process::Command,
};

use anyhow::Result;
use argparse::{ ArgumentParser, StoreTrue, Store };

fn github_user() -> &'static str {
	"r-o-b-o-t-o"
}

fn github_repository() -> &'static str {
	"soundboard"
}

#[derive(Debug, serde::Deserialize)]
struct GitHubUser {
	pub login: String,
	pub id: u64,
	pub node_id: String,
	pub avatar_url: String,
	pub gravatar_id: String,
	pub url: String,
	pub html_url: String,
	pub followers_url: String,
	pub following_url: String,
	pub gists_url: String,
	pub starred_url: String,
	pub subscriptions_url: String,
	pub organizations_url: String,
	pub repos_url: String,
	pub events_url: String,
	pub received_events_url: String,
	#[serde(rename = "type")]
	pub _type: String,
	pub site_admin: bool,
}

#[derive(Debug, serde::Deserialize)]
struct GitHubReleaseAsset {
	pub url: String,
	pub id: u64,
	pub node_id: String,
	pub name: String,
	pub label: Option<String>,
	pub uploader: GitHubUser,
	pub content_type: String,
	pub state: String,
	pub size: usize,
	pub download_count: u64,
	pub created_at: String,
	pub updated_at: String,
	pub browser_download_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
	pub url: String,
	pub assets_url: String,
	pub upload_url: String,
	pub html_url: String,
	pub id: u64,
	pub author: GitHubUser,
	pub node_id: String,
	pub tag_name: String,
	pub target_commitish: String,
	pub name: String,
	pub draft: bool,
	pub prerelease: bool,
	pub created_at: String,
	pub published_at: String,
	pub assets: Vec<GitHubReleaseAsset>,
	pub tarball_url: String,
	pub zipball_url: String,
	pub body: String,
}

pub fn is_update_available() -> Result<Option<String>, ureq::Error> {
	match get_latest_release() {
		Ok(latest_release) => {
			let current_version = env!("CARGO_PKG_VERSION");
			if !is_version_newer(current_version, &latest_release.tag_name) {
				return Ok(None);
			}

			let current_platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
			for asset in latest_release.assets {
				if asset.name.contains(&current_platform) {
					return Ok(Some(asset.browser_download_url));
				}
			}

			Ok(None)
		},
		Err(err) => Err(err),
	}
}

pub fn download_update(download_url: &str, quit_app_fn: impl Fn()) -> Result<()> {
	let resp = ureq::get(download_url)
		.call()?;

	if !resp.has("Content-Length") {
		return Err(anyhow::Error::msg("No Content-Length header on response"));
	}

	let len = resp.header("Content-Length")
		.and_then(|s| s.parse::<usize>().ok()).unwrap();

	let mut bytes: Vec<u8> = Vec::with_capacity(len);
	resp.into_reader()
		.read_to_end(&mut bytes)?;

	let url_split = download_url.split('/');
	let mut archive_path = std::env::temp_dir();
	let file_name = url_split.last().unwrap();
	archive_path.push(file_name);
	log::info!("Saving new version package to: {}", archive_path.display());

	std::fs::write(&archive_path, bytes)?;
	let mut target_dir = archive_path.clone();
	target_dir.set_extension("");
	if target_dir.exists() {
		std::fs::remove_dir_all(&target_dir)?;
	}
	extract_zip(&archive_path, &target_dir)?;
	std::fs::remove_file(archive_path)?;

	let mut updated_app_dir = target_dir;
	updated_app_dir.push("soundboard");

	let mut bin_path = updated_app_dir.clone();
	bin_path.push(format!("soundboard{}", std::env::consts::EXE_SUFFIX));

	let mut target_dir = std::env::current_exe()?;
	target_dir.pop();

	Command::new(bin_path)
		.current_dir(updated_app_dir)
		.args(&[
			"--copy-update",
			"--update-pid", &format!("{}", std::process::id()),
			"--copy-update-path", &format!("{}", target_dir.display()),
		])
		.spawn()?;

	quit_app_fn();
	Ok(())
}

fn get_latest_release() -> Result<GitHubRelease, ureq::Error> {
	let url = format!("https://api.github.com/repos/{}/{}/releases/latest", github_user(), github_repository());
	let agent = ureq::AgentBuilder::new()
		.timeout(Duration::from_secs(3))
		.build();
	match agent.get(&url).call() {
		Ok(response) => {
			let res_str = response.into_string()?;
			let latest_release: GitHubRelease = match serde_json::from_str(&res_str) {
				Ok(release) => release,
				Err(err) => {
					let err = std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Could not parse response JSON: {}", err));
					return Err(ureq::Error::from(err));
				},
			};
			Ok(latest_release)
		},
		Err(err) => Err(err),
	}
}

fn is_version_newer(current_version: &str, test_version: &str) -> bool {
	let current_version = current_version.replace("v", "");
	let current_split: Vec<&str> = current_version.split('.').collect();

	let test_version = test_version.replace("v", "");
	let test_split: Vec<&str> = test_version.split('.').collect();

	if current_split.len() != 3 || test_split.len() != 3 {
		return false;
	}

	match test_split[0].cmp(current_split[0]) {
		Ordering::Greater => true,
		Ordering::Less => false,
		Ordering::Equal => {
			match test_split[1].cmp(current_split[1]) {
				Ordering::Greater => true,
				Ordering::Less => false,
				Ordering::Equal => test_split[2] > current_split[2]
			}
		},
	}
}

fn create_command() -> Command {
	let mut cmd;
	if cfg!(target_os = "windows") {
		cmd = Command::new("cmd");
		cmd.arg("/C");
	} else {
		cmd = Command::new("sh");
		cmd.arg("-c");
	}
	cmd
}

fn extract_zip(archive_path: &PathBuf, target_dir: &PathBuf) -> Result<()> {
	let file = std::fs::File::open(archive_path)?;
	let mut archive = zip::ZipArchive::new(file)?;

	for i in 0..archive.len() {
		let mut file = archive.by_index(i)?;
		let outpath = match file.enclosed_name() {
			Some(path) => {
				let mut dir = PathBuf::from(target_dir);
				dir.push(path);
				dir
			},
			None => continue,
		};

		if (&*file.name()).ends_with('/') {
			std::fs::create_dir_all(&outpath)?;
		} else {
			if let Some(p) = outpath.parent() {
				if !p.exists() {
					std::fs::create_dir_all(&p)?;
				}
			}
			let mut outfile = std::fs::File::create(&outpath)?;
			std::io::copy(&mut file, &mut outfile)?;
		}

		// Get and Set permissions
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;

			if let Some(mode) = file.unix_mode() {
				fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
			}
		}
	}

	Ok(())
}

fn is_process_running(pid: u32) -> Result<bool> {
	if pid == 0 {
		return Err(anyhow::Error::msg("Pid is 0"));
	}

	let mut exit_code: u32 = 1;
	unsafe {
		let handle = winapi::um::processthreadsapi::OpenProcess(winapi::um::winnt::PROCESS_QUERY_INFORMATION, 0, pid);
		if handle.is_null() {
			let err = winapi::um::errhandlingapi::GetLastError();
			if err == winapi::shared::winerror::ERROR_INVALID_PARAMETER {
				// Invalid parameter means process does not exist
				return Ok(false);
			}
			return Err(anyhow::Error::msg(format!("Could not OpenProcess: {}", winapi::um::errhandlingapi::GetLastError())));
		}

		let res = winapi::um::processthreadsapi::GetExitCodeProcess(handle, &mut exit_code);
		winapi::um::handleapi::CloseHandle(handle);
		if res == 0 {
			return Err(anyhow::Error::msg(format!("Could not GetExitCodeProcess: {}", winapi::um::errhandlingapi::GetLastError())));
		}
		Ok(exit_code == winapi::um::minwinbase::STILL_ACTIVE)
	}
}

fn copy_update(ppid: u32, target_dir: PathBuf) -> Result<()> {
	while is_process_running(ppid)? {
		thread::sleep(Duration::from_millis(500));
	}

	let mut target_bin = target_dir.clone();
	target_bin.push(format!("soundboard{}", std::env::consts::EXE_SUFFIX));
	crate::utils::retry(|| {
		std::fs::copy(std::env::current_exe().unwrap(), &target_bin)
	}, 5, std::time::Duration::from_millis(500))?;

	let mut target_res = target_dir;
	target_res.push("resources");
	std::fs::remove_dir_all(&target_res)?;

	let mut updated_res = std::env::current_exe()?;
	updated_res.pop();
	updated_res.push("resources");
	let mut cmd;
	if cfg!(target_os = "windows") {
		cmd = create_command();
		cmd.args(&["ROBOCOPY", "/E", "/NFL", "/NDL", "/NJH", "/NJS", "/NP", "/NS", "/NC", &format!("{}", updated_res.display()), &format!("{}", target_res.display())]);
	} else {
		cmd = create_command();
		cmd.args(&["cp", "-r", &format!("{}", updated_res.display()), &format!("{}", target_res.display())]);
	}
	cmd.status()?;

	Ok(())
}

fn start_updated_app(target_dir: PathBuf) -> Result<()> {
	let mut target_bin = target_dir;
	target_bin.push(format!("soundboard{}", std::env::consts::EXE_SUFFIX));

	let mut current_dir = std::env::current_exe()?;
	current_dir.pop();
	current_dir.pop();

	start_for_cleanup(target_bin, current_dir)?;

	Ok(())
}

fn start_for_cleanup(bin: PathBuf, dir: PathBuf) -> Result<()> {
	let mut start_dir = bin.clone();
	start_dir.pop();

	let mut cmd = Command::new(bin);
	cmd
		.args(&[
			"--update-pid", &format!("{}", std::process::id()),
			"--clean-update", &format!("{}", dir.display()),
		])
		.current_dir(start_dir);
	cmd.spawn()?;

	Ok(())
}

pub fn clean_update(pid: u32, dir: PathBuf) -> Result<()> {
	while is_process_running(pid)? {
		thread::sleep(Duration::from_millis(500));
	}

	crate::utils::retry(|| {
		std::fs::remove_dir_all(&dir)
	}, 5, Duration::from_millis(500))?;

	Ok(())
}

pub fn check_args() {
	let mut do_copy_update = false;
	let mut update_pid: u32 = 0;
	let mut copy_update_path = String::new();
	let mut clean_update_path = String::new();

	{
		let mut ap = ArgumentParser::new();
		ap.refer(&mut do_copy_update)
			.add_option(&["--copy-update"], StoreTrue, "Copy the updated app to a directory specified with --copy-update-path");
		ap.refer(&mut update_pid)
			.add_option(&["--update-pid"], Store, "Wait before a process ends before continuing the update process");
		ap.refer(&mut copy_update_path)
			.add_option(&["--copy-update-path"], Store, "Target directory to copy the updated app in");
		ap.refer(&mut clean_update_path)
			.add_option(&["--clean-update"], Store, "Clean up the temporary directory used to update the app");
		let _ = ap.parse_args();
	}

	if do_copy_update && update_pid != 0 && !copy_update_path.is_empty() {
		let copy_update_path = std::path::PathBuf::from(copy_update_path);
		if let Err(err) = copy_update(update_pid, copy_update_path.clone()) {
			log::error!("Could not copy updated app: {}", err);
			std::process::exit(1);
		}
		if let Err(err) = start_updated_app(copy_update_path) {
			log::error!("Could not start updated app: {}", err);
			std::process::exit(1);
		} else {
			std::process::exit(0);
		}
	}
	if !clean_update_path.is_empty() && update_pid != 0 {
		let clean_update_path = std::path::PathBuf::from(clean_update_path);
		if let Err(err) = clean_update(update_pid, clean_update_path) {
			log::error!("Could not clean update: {}", err);
			std::process::exit(1);
		}
	}
}



#[test]
fn test_is_version_newer() {
	let current_version = "v0.0.5";
	assert!(!is_version_newer(current_version, "v0.0.4"));
	assert!(!is_version_newer(current_version, "v0.0.5"));
	assert!(is_version_newer(current_version, "v0.0.6"));
	assert!(is_version_newer(current_version, "v0.1.0"));
	assert!(is_version_newer(current_version, "v1.0.0"));

	let current_version = "v0.5.0";
	assert!(!is_version_newer(current_version, "v0.0.6"));
	assert!(!is_version_newer(current_version, "v0.4.0"));
	assert!(!is_version_newer(current_version, "v0.5.0"));
	assert!(is_version_newer(current_version, "v0.6.0"));
	assert!(is_version_newer(current_version, "v0.5.1"));
	assert!(is_version_newer(current_version, "v1.0.0"));

	let current_version = "v5.0.0";
	assert!(!is_version_newer(current_version, "v0.6.0"));
	assert!(!is_version_newer(current_version, "v0.0.6"));
	assert!(!is_version_newer(current_version, "v4.0.0"));
	assert!(!is_version_newer(current_version, "v5.0.0"));
	assert!(is_version_newer(current_version, "v6.0.0"));
	assert!(is_version_newer(current_version, "v5.1.0"));
	assert!(is_version_newer(current_version, "v5.0.1"));
}
