use std::convert::Infallible;
use std::net::SocketAddr;

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use tokio::net::TcpListener;

mod support;
use support::TokioIo;

async fn shutdown_signal() {
    // 等待 CTRL+C 信号
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

async fn hello(_: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    println!("Hello world! begin");
   
    // 等待 10 秒
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
   
    println!("Hello world! end");
   
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    // 指定我们的 HTTP 设置（http1、http2、自动全部工作）
    let http = http1::Builder::new();
    // 优雅的关机观察者
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    // 当该信号完成时，开始关闭
    let mut signal = std::pin::pin!(shutdown_signal());

    // 我们的服务器接受循环
    loop {
        tokio::select! {
            Ok((stream, _addr)) = listener.accept() => {
                let io = TokioIo::new(stream);
                let conn = http.serve_connection(io, service_fn(hello));
                // 此连接添加入观察中
                let fut = graceful.watch(conn);
                tokio::spawn(async move {
                    if let Err(e) = fut.await {
                        eprintln!("Error serving connection: {:?}", e);
                    }
                });
            },

            _ = &mut signal => {
                eprintln!("graceful shutdown signal received");
                // 停止接受循环
                break;
            }
        }
    }

    // 现在开始关闭并等待它们完成
    // 可选：启动超时来限制等待时间。
    tokio::select! {
        // 等待所有连接完成
        _ = graceful.shutdown() => {
            eprintln!("all connections gracefully closed");
        },
        // 超时
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            eprintln!("timed out wait for all connections to close");
        }
    }

    Ok(())
    // 等待所有连接完成
}
