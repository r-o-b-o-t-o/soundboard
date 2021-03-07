use web_view::Content;

pub fn start_settings_window() {
	let window = web_view::builder()
		.title("Settings")
		.content(Content::Url("http://127.0.0.1:29815/settings.html"))
		.size(1280, 960)
		.resizable(true)
		.frameless(false)
		.visible(true)
		.debug(true)
		.user_data(())
		.invoke_handler(|_webview, _arg| Ok(()));
	window.run().unwrap();

	/*std::thread::spawn(move || {
		window.run().unwrap();
	});*/
}
