pub mod client;

use client_app::client::Client;

use clap::Parser;
use std::io::Write;

fn input_username() -> std::io::Result<String> {
    loop {
        print!("Enter username to join the chat: ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().is_empty() {
            tracing::warn!("Username cannot be empty. Try again.");
            continue;
        }

        break Ok(input.trim_end().to_string());
    }
}

#[derive(Parser)]
#[command(name = "client")]
#[command(about = "A simple CLI messenger client", long_about = None)]
struct Args {
    #[arg(long)]
    host: String,

    #[arg(short, long)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let username = match input_username() {
        Err(e) => {
            tracing::error!("Failed to read username.");
            return Err(e.into());
        }
        Ok(username) => username,
    };

    let addr = format!("{}:{}", args.host, args.port).parse()?;
    let client = Client::connect(addr, username).await?;
    client.run().await?;

    Ok(())
}
