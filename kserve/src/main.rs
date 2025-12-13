use std::path::PathBuf;

use actix_web::{App, HttpResponse, HttpServer, web};
use anyhow::Context;

pub mod dir;
pub use dir::Dir;

pub mod frame;
pub use frame::Frame;

pub mod config;
pub use config::Config;

const BIND: (&str, u16) = const {
	let port = if cfg!(debug_assertions) { 9090 } else { 9090 };
	("0.0.0.0", port)
};

// #[get("/")]
// async fn index() -> impl Responder {
// 	"hello index"
// }

async fn root_fallback(
	path: web::Path<String>,
	dir: web::Data<Dir>,
	config: web::Data<Config>,
) -> HttpResponse {
	let path = path.into_inner();
	let path = if path.is_empty() { ".".into() } else { path };

	let path = PathBuf::from(path);
	let res = dir.handle_path(&path, config.as_ref()).await;

	match res {
		// Ok((mime, a)) => HttpResponse::Ok().content_type(mime.as_ref()).body(a),
		Ok(resp) => resp,
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
	let dir_path = dir_path
		.canonicalize()
		.with_context(|| format!("while canonicalizing {}", dir_path.display()))?;

	let dir = Dir::new(dir_path);
	let dir = web::Data::new(dir);

	let config = dir.config().await?;
	println!("{config:#?}");
	let config = web::Data::new(config);

	let server = HttpServer::new(move || {
		App::new()
			.app_data(dir.clone())
			.app_data(config.clone())
			// .service(index)
			.route("/{tail:.*}", web::get().to(root_fallback))
	})
	.bind(BIND)?
	.run();
	println!("listening on http://{}:{}", BIND.0, BIND.1);

	server.await?;
	Ok(())
}
