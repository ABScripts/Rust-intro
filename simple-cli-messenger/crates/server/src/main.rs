use server_app::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let server = Server::bind("0.0.0.0:3456").await?;
    server.run().await
}
