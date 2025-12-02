use actix_web::{App, HttpServer, Responder, get, web};
use tokio::sync::Mutex;

const BIND: (&str, u16) = const {
	let port = if cfg!(debug_assertions) { 9090 } else { 9090 };
	("0.0.0.0", port)
};

#[get("/")]
async fn index() -> impl Responder {
	"hello index"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let server = HttpServer::new(move || App::new().service(index))
		.bind(BIND)?
		.run();
	println!("listening on http://{}:{}", BIND.0, BIND.1);

	server.await?;
	Ok(())
}
