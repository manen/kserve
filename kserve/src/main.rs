use std::path::PathBuf;

use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use anyhow::Context;
use tokio::sync::Mutex;

pub mod dir;
pub use dir::Dir;

const BIND: (&str, u16) = const {
	let port = if cfg!(debug_assertions) { 9090 } else { 9090 };
	("0.0.0.0", port)
};

#[get("/")]
async fn index() -> impl Responder {
	"hello index"
}

async fn root_fallback(path: web::Path<String>, dir: web::Data<Dir>) -> HttpResponse {
	let path = path.into_inner();
	let path = PathBuf::from(path);
	let res = dir.md_to_html(&path).await;

	match res {
		Ok(a) => HttpResponse::Ok().content_type("text/html").body(a),
		Err(err) => {
			let err = format!("{err}\n\n{err:#?}");
			HttpResponse::BadRequest().body(err)
		}
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let mut args = std::env::args();
	let _ = args.next();

	let dir_path = args
		.next()
		.map(|s| Ok(PathBuf::from(s)))
		.unwrap_or_else(std::env::current_dir)
		.with_context(|| "while getting dir path")?;

	let dir = Dir::new(dir_path);
	let dir = web::Data::new(dir);

	let server = HttpServer::new(move || {
		App::new()
			.app_data(dir.clone())
			.service(index)
			.route("/{tail:.*}", web::get().to(root_fallback))
	})
	.bind(BIND)?
	.run();
	println!("listening on http://{}:{}", BIND.0, BIND.1);

	server.await?;
	Ok(())
}
