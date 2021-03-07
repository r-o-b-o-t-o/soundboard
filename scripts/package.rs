use std::process::Command;

mod zip_package;

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

fn check_versions() {
	let package_json = std::fs::read_to_string("soundboard-ui/package.json").unwrap();
	let package_json: serde_json::Value = serde_json::from_str(&package_json).unwrap();
	let rust_version = env!("CARGO_PKG_VERSION");
	let ui_version = package_json["version"].as_str().unwrap();
	assert_eq!(rust_version, ui_version);
}

fn build() {
	Command::new("cargo")
		.args(&["build", "--release"])
		.status()
		.expect("Failed to run cargo build");

	let lint_status = create_command()
		.current_dir("soundboard-ui")
		.args(&["npm", "run", "lint"])
		.status()
		.expect("Failed to run npm run lint");
	if !lint_status.success() {
		std::process::exit(1);
	}

	create_command()
		.current_dir("soundboard-ui")
		.args(&["npm", "start"])
		.status()
		.expect("Failed to run npm start");
}

fn clean_package_dir() {
	let mut cmd;
	if cfg!(target_os = "windows") {
		cmd = create_command();
		cmd.args(&["RMDIR", "/S", "/Q", ".\\package"]);
	} else {
		cmd = create_command();
		cmd.args(&["rm", "-rf", "./package"]);
	}
	cmd
		.status()
		.expect("Could not clean package directory");
}

fn create_package_dir() {
	std::fs::create_dir("package").expect("Could not create package dir");
	std::fs::create_dir("package/soundboard").expect("Could not create package/soundboard dir");

	std::fs::copy(
		format!("target/release/soundboard{}", std::env::consts::EXE_SUFFIX),
		format!("package/soundboard/soundboard{}", std::env::consts::EXE_SUFFIX)
	).expect("Could not copy binary to package dir");

	let mut cmd;
	if cfg!(target_os = "windows") {
		cmd = create_command();
		cmd.args(&["ROBOCOPY", "/E", "/NFL", "/NDL", "/NJH", "/NJS", "/NP", "/NS", "/NC", ".\\soundboard-ui\\dist", ".\\package\\soundboard\\resources"]);
	} else {
		cmd = create_command();
		cmd.args(&["cp", "-r", "./soundboard-ui/dist", "./package/soundboard/resources"]);
	}
	cmd
		.status()
		.expect("Could not copy resources directory");
}

fn create_package_archive() {
	zip_package::main().expect("Could not zip package directory");
}

fn main() {
	check_versions();
	build();
	clean_package_dir();
	create_package_dir();
	create_package_archive();
	clean_package_dir();
}
