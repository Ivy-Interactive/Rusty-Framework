mod app_shell;
mod apps;
mod sample_base;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    server::SamplesServer::run(5050).await
}

#[cfg(test)]
mod tests {
    use rusty::prelude::*;

    use crate::app_shell::AppShell;
    use crate::apps::all_apps;

    #[tokio::test]
    async fn test_server_starts_and_serves_websocket() {
        let server = RustyServer::new(0, || AppShell);
        let addr = server.serve_background().await.unwrap();
        // Verify the server is listening by connecting via TCP
        let stream = tokio::net::TcpStream::connect(addr).await;
        assert!(stream.is_ok(), "Server should be accepting connections");
    }

    #[tokio::test]
    async fn test_all_apps_can_build() {
        let mut runtime = Runtime::new(AppShell);
        let _tree = runtime.build().await;
        // If we get here without panicking, the AppShell and all app factories were called
        assert!(all_apps().len() >= 15);
    }
}
