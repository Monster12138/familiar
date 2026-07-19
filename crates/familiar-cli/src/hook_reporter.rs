use anyhow::Result;
use std::io::Read;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

/// Reads data from stdin and forwards it to a Unix socket
pub async fn forward_stdin_to_socket(socket_path: &str) -> Result<()> {
    let mut stdin = std::io::stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer)?;

    let mut stream = UnixStream::connect(socket_path).await?;
    stream.write_all(&buffer).await?;
    stream.flush().await?;

    Ok(())
}
