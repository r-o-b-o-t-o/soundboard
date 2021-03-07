fn main() {
	if cfg!(target_os = "windows") {
		let mut res = winres::WindowsResource::new();
		res.set_icon("soundboard-ui/static/favicon.ico");
		res.compile().unwrap();
	}
}
