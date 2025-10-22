//! Simplified server module for demonstration purposes

use std::net::SocketAddr;
use tokio::net::TcpListener;
use log::{info, error};
use hyper_util::server::conn::auto;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use bytes::Bytes;
use hyper::service::service_fn;

/// Simple HTTP proxy server for testing
pub struct SimpleServer {
    addr: SocketAddr,
}

impl SimpleServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("Simple proxy server listening on: {}", self.addr);

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            info!("New connection from: {}", remote_addr);

            // Handle connection in a simple way for demo
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                if let Err(e) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .http1()
                    .serve_connection(
                        io,
                        service_fn(|req| async move {
                            simple_handle_request(req).await
                        })
                    )
                    .await
                {
                    error!("Connection error: {}", e);
                }
            });
        }
    }
}

async fn simple_handle_request(
    _req: Request<hyper::body::Incoming>
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let response_body = r#"{
        "status": "ok",
        "message": "Proxy.rs Simple Server",
        "version": "0.3.7",
        "features": [
            "Proxy Discovery",
            "Quality Testing",
            "Connection Pooling",
            "DNSBL Security"
        ]
    }"#;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_body)))
        .unwrap())
}

/// Start simple server for testing
pub async fn start_simple_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = "127.0.0.1:8080".parse()?;
    let server = SimpleServer::new(addr);
    server.run().await
}