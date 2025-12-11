mod client;
mod game;
mod game_manager;
mod server;

use server::ServerApp;
use tokio::time::{self, Duration, MissedTickBehavior};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const TICK_INTERVAL: Duration = Duration::from_micros(33_333); // ≈30 Hz

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();

    let mut app = ServerApp::new()?;

    let mut ticker = time::interval(TICK_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
            _ = ticker.tick() => {
                if let Err(err) = app.tick() {
                    error!(error = %err, "Tick failed");
                }
            }
        }
    }

    app.shutdown();
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
