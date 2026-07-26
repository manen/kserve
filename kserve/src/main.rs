use std::path::PathBuf;

use actix_web::{App, HttpResponse, HttpServer, web};
use anyhow::Context;

pub mod dir;
pub use dir::Dir;

pub mod frame;
pub use frame::Frame;

pub mod config;
pub use config::Config;

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

	let default_path = || {
		let cwd = std::env::current_dir()?;
		let config_path = cwd.join("_kserve.toml");
		std::io::Result::Ok(config_path)
	};

	let config_path = args
		.next()
		.map(|s| Ok(PathBuf::from(s)))
		.unwrap_or_else(default_path)
		.with_context(|| "while getting dir path")?;

	let config = Config::new(&config_path)
		.await
		.with_context(|| format!("while reading config from {}", config_path.display()))?;

	// config is read, time to set up the serving directory

	let dir_path_maybe_relative = PathBuf::from(config.serve_directory.as_ref());
	let dir_path = std::fs::canonicalize(&dir_path_maybe_relative).with_context(|| {
		format!(
			"while canonicalizing the dir path {}",
			dir_path_maybe_relative.display()
		)
	})?;

	let dir = Dir::new(dir_path);
	let dir = web::Data::new(dir);

	println!("{config:#?}");
	let config = web::Data::new(config);
	let bind = (config.addr.to_owned(), config.port);

	let server = HttpServer::new(move || {
		App::new()
			.app_data(dir.clone())
			.app_data(config.clone())
			// .service(index)
			.route("/{tail:.*}", web::get().to(root_fallback))
	})
	.bind((bind.0.as_ref(), bind.1))?
	.run();
	println!("listening on http://{}:{}", bind.0, bind.1);

	server.await?;
	Ok(())
}
