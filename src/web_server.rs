use anyhow::Result;
use actix_files::Files;
use actix_web::HttpServer;

#[derive(Default)]
pub struct WebServer {
	server: Option<actix_web::dev::Server>,
}

impl WebServer {
	pub fn start(&mut self) -> Result<()> {
		let address = "127.0.0.1:29815";
		let path = crate::App::get_web_resources_directory().expect("Could not get web resources directory");

		self.server = Some(HttpServer::new(move || {
			actix_web::App::new().service(Files::new("/", path.clone()))
		})
		.workers(3)
		.bind(address)?
		.run());

		log::info!("Static web server running on {}", address);

		Ok(())
	}

	pub async fn stop(&self) {
		log::info!("Stopping web server...");
		self.server.as_ref().unwrap().stop(true).await;
	}
}
