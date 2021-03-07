use std::path::PathBuf;

use winapi::um::winuser;

pub fn setup_panic_hook() {
	let orig_hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |panic_info| {
		// Invoke the default handler and exit the process
		log::error!("{}", panic_info);
		orig_hook(panic_info);
		std::process::exit(1);
	}));
}

pub fn retry<R, E>(f: impl Fn() -> Result<R, E>, max_retries: u16, sleep_dur: std::time::Duration) -> Result<R, E> {
	let mut n_retries = 0;

	loop {
		match f() {
			Ok(r) => return Ok(r),
			Err(err) => {
				n_retries += 1;
				if n_retries >= max_retries {
					return Err(err);
				}
				std::thread::sleep(sleep_dur);
			},
		}
	}
}

pub fn get_monitor_size() -> (i32, i32) {
	unsafe {
		(
			winuser::GetSystemMetrics(winuser::SM_CXSCREEN),
			winuser::GetSystemMetrics(winuser::SM_CYSCREEN),
		)
	}
}

pub fn path_is_child(parent: &PathBuf, child: &PathBuf) -> std::io::Result<bool> {
	let parent = parent.canonicalize()?;
	let child = child.canonicalize()?;
	Ok(child.starts_with(parent))
}
