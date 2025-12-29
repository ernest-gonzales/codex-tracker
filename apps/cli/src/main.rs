mod args;
mod config;
mod dirs;

use std::io;
use std::net::SocketAddr;
use std::process::Command;

use app_api::AppContext;
use http_api::{HttpState, generate_csrf_token};
use tracker_app::{AppPaths, AppState, ensure_app_data_dir, migrate_legacy_storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::parse_args().map_err(|err| {
        eprintln!("{err}");
        args::print_help();
        io::Error::new(io::ErrorKind::InvalidInput, "invalid arguments")
    })?;

    let config = config::load_or_create().map_err(io::Error::other)?;
    if config.created {
        println!(
            "Created config at {} (default port {}).",
            config.paths.file.display(),
            config.config.port
        );
    }

    let data_dir = dirs::resolve_data_dir().map_err(io::Error::other)?;
    if data_dir.matched_existing {
        println!("Using existing data dir: {}", data_dir.dir.display());
    } else {
        println!("Using data dir: {}", data_dir.dir.display());
    }

    let port = args.port.unwrap_or(config.config.port);

    let paths = AppPaths::new(data_dir.dir.clone());
    ensure_app_data_dir(&paths)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
    let legacy_backup_dir = migrate_legacy_storage(&paths)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    let app_state = AppState::new(paths.db_path, paths.pricing_defaults_path);
    let is_fresh_db = app_state.is_fresh_db();
    if let Err(err) = app_state.setup_db() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("failed to initialize database: {}", err),
        )
        .into());
    }
    if is_fresh_db {
        if let Err(err) = app_state.apply_pricing_defaults() {
            eprintln!("failed to apply pricing defaults: {}", err);
        }
    }
    if let Err(err) = app_state.sync_pricing_defaults() {
        eprintln!("failed to sync pricing defaults: {}", err);
    }

    let ingest_state = app_state.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(err) = ingest_state.services.ingest.run() {
            eprintln!("failed to refresh data on startup: {}", err);
        }
    });

    let context = AppContext {
        app_state,
        app_data_dir: data_dir.dir,
        legacy_backup_dir,
    };

    let csrf_token = generate_csrf_token();
    let state = HttpState::new(context, csrf_token);
    let router = http_api::router(state);

    let (listener, actual_port, used_fallback) = bind_port(port).await?;
    let url = format!("http://127.0.0.1:{actual_port}");

    if used_fallback {
        eprintln!("Configured port {port} was unavailable; using {actual_port} for this run.");
    }

    println!("Codex Tracker is running at {url}");
    println!("Press Ctrl+C to stop.");

    if !args.no_open {
        if let Err(err) = open_url(&url) {
            eprintln!("failed to open browser: {}", err);
        }
    }

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn bind_port(port: u16) -> Result<(tokio::net::TcpListener, u16, bool), io::Error> {
    if port == 0 {
        let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let actual_port = listener.local_addr()?.port();
        return Ok((listener, actual_port, false));
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => Ok((listener, port, false)),
        Err(_) => {
            let listener =
                tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
            let actual_port = listener.local_addr()?.port();
            Ok((listener, actual_port, true))
        }
    }
}

fn open_url(url: &str) -> Result<(), io::Error> {
    let status = Command::new("open").arg(url).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "open command failed"))
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
