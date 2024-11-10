#![deny(warnings)]

use std::convert::Infallible;
use std::net::SocketAddr;

use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use tokio::net::TcpListener;

mod support;

use support::{TokioIo, TokioTimer};

// 一个异步函数，它消耗一个请求，不对其执行任何操作并返回一个
// 回复。
async fn hello(_: Request<impl hyper::body::Body>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello World!"))))
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 漂亮的环境记录器::init();

    // 这个地址是本地主机
    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

    // 绑定到端口并侦听传入的 TCP 连接
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        // 当接收到传入的 TCP 连接时，抓取 TCP 流
        // 客户端<->服务器通信。
        //
        // 注意，这是一个 .await 点，这个循环将永远循环，但不是一个繁忙的循环。这
        // .await 点允许 Tokio 运行时将任务从线程中拉出，直到任务完成
        // 有工作要做。在这种情况下，连接到达我们正在侦听的端口，并且
        // 任务被唤醒，此时任务被放回到线程上，并且
        // 由运行时驱动向前，最终产生 TCP 流。
        let (tcp, _) = listener.accept().await?;

        // 使用适配器访问实现 `tokio::io` 特征的东西，就像它们实现一样
        // `hyper::rt` IO 特征。
        let io = TokioIo::new(tcp);

        // 在 Tokio 中启动一个新任务，以便我们可以继续侦听新的 TCP 连接
        // 当前任务，无需等待我们刚刚收到的 HTTP1 连接的处理
        // 完成
        tokio::task::spawn(async move {
            
            // 使用 HTTP1 处理来自客户端的连接并传递任何
            // 在与“hello”函数的连接上收到的 HTTP 请求
            if let Err(err) = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service_fn(hello))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
