use std::{convert::Infallible, net::SocketAddr};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    Request, Response,
};
use http_body_util::Full;
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use tokio::net::TcpListener;
use tower::ServiceBuilder;

mod logger_ext;
use logger_ext::Logger;

async fn hello(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

// hyper_util::service::TowerToHyperService trait is an adapter to convert tower Service to hyper Service.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::spawn(async move {
            // N.B. should use tower service_fn here, since it's required to be implemented tower Service trait before convert to hyper Service!
            let svc = tower::service_fn(hello);
            let svc = ServiceBuilder::new().layer_fn(Logger::new).service(svc);
            // Convert it to hyper service
            let svc = TowerToHyperService::new(svc);
            if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                eprintln!("server error: {}", err);
            }
        });
    }
}