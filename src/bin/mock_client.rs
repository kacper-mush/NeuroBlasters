use anyhow::Result;
use rust_big::client::mock::{ClientArgs, init_tracing, run};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = ClientArgs::parse()?;
    run(args).await
}
