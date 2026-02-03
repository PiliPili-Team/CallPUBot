#[tokio::main]
async fn main() -> anyhow::Result<()> {
    callpu::run().await
}
