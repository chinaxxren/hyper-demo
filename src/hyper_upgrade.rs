#![deny(warnings)]

// 注意：“hyper::upgrade”文档链接到此升级。
use std::net::SocketAddr;
use std::str;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

use bytes::Bytes;
use http_body_util::Empty;
use hyper::header::{HeaderValue, UPGRADE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Request, Response, StatusCode};

mod support;
use support::TokioIo;

// 一个简单的类型别名以便DRY。
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// HTTP 升级后处理服务器端 I/O。
async fn server_upgraded_io(upgraded: Upgraded) -> Result<()> {
    let mut upgraded = TokioIo::new(upgraded);
    // 我们有一个升级的连接，我们可以读取和
    // 直接写上去。
    //
    // 因为我们完全控制这个例子，所以我们确切地知道
    // 客户端将写入多少字节，因此只需准确读取...
    let mut vec = vec![0; 7];
    upgraded.read_exact(&mut vec).await?;
    println!("server[foobar] recv: {:?}", str::from_utf8(&vec));

    // 现在写回服务器“foobar”协议
    // 回复...
    upgraded.write_all(b"bar=foo").await?;
    println!("server[foobar] sent");
    Ok(())
}

/// 我们的服务器 HTTP 处理程序用于启动 HTTP 升级。
async fn server_upgrade(mut req: Request<hyper::body::Incoming>) -> Result<Response<Empty<Bytes>>> {
    let mut res = Response::new(Empty::new());

    // 向任何没有的请求发送 400
    // 一个“升级”标头。
    if !req.headers().contains_key(UPGRADE) {
        *res.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(res);
    }

    // 设置一个最终将获得升级的未来
    // 连接并谈论新协议，孕育未来
    // 进入运行时。
    //
    // 注意：在 101 响应之前，这不可能实现
    // 在下面返回，所以最好生成这个 future
    // 等待它完成然后返回响应。
    tokio::task::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {
                if let Err(e) = server_upgraded_io(upgraded).await {
                    eprintln!("server foobar io error: {}", e)
                };
            }
            Err(e) => eprintln!("upgrade error: {}", e),
        }
    });

    // 现在返回 101 响应，表示我们同意升级到某些
    // 虚构的“foobar”协议。
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    res.headers_mut()
        .insert(UPGRADE, HeaderValue::from_static("foobar"));
    Ok(res)
}

/// HTTP 升级后处理客户端 I/O。
async fn client_upgraded_io(upgraded: Upgraded) -> Result<()> {
    let mut upgraded = TokioIo::new(upgraded);
    // 我们已经获得了可以读取的升级连接
    // 并直接写在上面。让我们开始“foobar”协议。
    upgraded.write_all(b"foo=bar").await?;
    println!("client[foobar] sent");

    let mut vec = Vec::new();
    upgraded.read_to_end(&mut vec).await?;
    println!("client[foobar] recv: {:?}", str::from_utf8(&vec));

    Ok(())
}

/// 我们的客户端 HTTP 处理程序用于启动 HTTP 升级。
/// 异步函数：客户端发起升级请求
/// 
/// 该函数尝试连接到指定地址，并发送一个HTTP请求，请求升级到自定义协议。
/// 如果服务器同意升级，它将处理升级后的连接。
/// 
/// # 参数
/// * `addr` - 目标服务器的Socket地址
/// 
/// # 返回
/// * `Result<()>` - 表示操作成功或包含错误的Result类型
async fn client_upgrade_request(addr: SocketAddr) -> Result<()> {
    // 创建HTTP请求
    let req = Request::builder()
        .uri(format!("http://{}/", addr))
        .header(UPGRADE, "foobar")
        .body(Empty::<Bytes>::new())
        .unwrap();

    // 连接到服务器
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    // 与服务器进行HTTP/1握手
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // 在后台任务中处理连接，以支持升级
    tokio::task::spawn(async move {
        // 不要忘记启用连接升级。
        if let Err(err) = conn.with_upgrades().await {
            println!("Connection failed: {:?}", err);
        }
    });

    // 发送升级请求
    let res = sender.send_request(req).await?;

    // 检查服务器是否同意升级
    if res.status() != StatusCode::SWITCHING_PROTOCOLS {
        panic!("Our server didn't upgrade: {}", res.status());
    }

    // 处理升级后的连接
    match hyper::upgrade::on(res).await {
        Ok(upgraded) => {
            // 处理升级后的IO
            if let Err(e) = client_upgraded_io(upgraded).await {
                eprintln!("client foobar io error: {}", e)
            };
        }
        Err(e) => eprintln!("upgrade error: {}", e),
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // 对于这个例子，我们只创建一个服务器和我们自己的客户端来交谈
    // 它，所以确切的端口并不重要。相反，让操作系统给我们一个
    // 未使用的端口。
    let addr: SocketAddr = ([127, 0, 0, 1], 0).into();

    let listener = TcpListener::bind(addr).await.expect("failed to bind");

    // 我们需要为客户端分配地址来发送消息。
    let addr = listener.local_addr().unwrap();

    // 对于此示例，使用一次性信号表示在 1 个请求之后，
    // 服务器应该关闭。
    let (tx, mut rx) = watch::channel(false);

    // 在默认执行器上生成服务器，
    // 这通常是 tokio 默认运行时的线程池。
    tokio::task::spawn(async move {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (stream, _) = res.expect("Failed to accept");
                    let io = TokioIo::new(stream);

                    let mut rx = rx.clone();
                    tokio::task::spawn(async move {
                        let conn = http1::Builder::new().serve_connection(io, service_fn(server_upgrade));

                        // 不要忘记启用连接升级。
                        let mut conn = conn.with_upgrades();

                        let mut conn = Pin::new(&mut conn);

                        tokio::select! {
                            res = &mut conn => {
                                if let Err(err) = res {
                                    println!("Error serving connection: {:?}", err);
                                    return;
                                }
                            }
                            
                            // 启用正常关闭后继续轮询连接。
                            _ = rx.changed() => {
                                conn.graceful_shutdown();
                            }
                        }
                    });
                }
                _ = rx.changed() => {
                    break;
                }
            }
        }
    });

    // 客户端请求 HTTP 连接升级。
    let request = client_upgrade_request(addr.clone());
    if let Err(e) = request.await {
        eprintln!("client error: {}", e);
    }

    // 完成 oneshot 以使服务器停止
    // 监听并且进程可以关闭。
    let _ = tx.send(true);
}