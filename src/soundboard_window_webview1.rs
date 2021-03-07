use web_view::Content;

pub fn start_soundboard_window() {
	let window = web_view::builder()
		.title("Soundboard")
		.content(Content::Url("http://127.0.0.1:29815/soundboard.html"))
		.size(1024, 768)
		.resizable(true)
		//.frameless(true)
		.visible(false)
		.debug(true)
		.user_data(())
		.invoke_handler(|_webview, _arg| Ok(()));

	std::thread::spawn(move || {
		window.run().unwrap();
	});
}
