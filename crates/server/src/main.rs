use clap::Parser;
use mg_server::{Config, ServeError, serve};

// musl's allocator is slow under contention; mimalloc keeps the static binary fast.
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> std::process::ExitCode {
    let config = Config::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads.unwrap_or(cores.min(2)).max(1))
        .enable_all()
        .build();
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "cannot start the runtime");
            return std::process::ExitCode::FAILURE;
        }
    };

    let result = runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(config.listen)
            .await
            .map_err(ServeError::Io)?;
        serve(config, listener, shutdown_signal()).await
    });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "server failed");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Ctrl+C everywhere, plus SIGTERM on Unix (systemd stops services with SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "cannot listen for Ctrl+C");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
