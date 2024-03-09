use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::types::ClientEvent;
use crate::types::Clients;
use crate::types::Command;
use crate::ui_client::UiWsWorker;

pub const CLIENT_ID: AtomicU64 = AtomicU64::new(1);

pub fn serve_index() -> Response<Full<Bytes>>  {
    let str = format!(r#"
<html>
    <head>
        <title>Your app</title>
    </head>
    <body>
        <script src="/index.js"></script>
    </body>
</html>"#, );

    Response::new(Full::new(Bytes::from(str)))
}

const INDEX_JS_BYTES: &[u8] = include_bytes!("../../dist/index.js");

struct Ctx {
    event_tx: mpsc::UnboundedSender<ClientEvent>,
    clients: Clients
}

async fn handle_req(mut req: Request<hyper::body::Incoming>, ctx: Ctx) -> Result<Response<Full<Bytes>>, hyper::Error> {
    log::info!("{} {}", req.method(), req.uri().path());

    if req.uri().path() == "/ws" && hyper_tungstenite::is_upgrade_request(&req) {
        log::info!("upgrade to websocket");
        let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None).unwrap();
        log::info!("websocket upgraded");
        let id = CLIENT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as usize;

        log::info!("websocket worker created");
        tokio::spawn(async move {
            let ws = websocket.await.unwrap();
            let worker = UiWsWorker::new(
                id, 
                ws, 
                ctx.event_tx.clone(),
                ctx.clients.clone()
            ).await;
            worker.run().await;
        });
        return Ok(response);
    }

    match req.uri().path() {
        "/index.js" => {
            Ok(Response::new(Full::new(Bytes::from(INDEX_JS_BYTES))))
        },
        _ => {
            Ok(serve_index())
        }
    }
}

pub async fn server(event_tx: mpsc::UnboundedSender<ClientEvent>, clients: Clients) {
    let addr = SocketAddr::from(([0, 0, 0, 0], 4477));
    let listener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(socket);
        let event_tx = event_tx.clone();
        let clients = clients.clone();
        tokio::spawn(async move {
            let service = service_fn(move |req| {
                // async move { Ok::<_, Error>(Response::new(Body::from(format!("Request #{}", value)))) }
                handle_req(req, Ctx { 
                    event_tx: event_tx.clone(),
                    clients: clients.clone()
                })
            });

            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service)
                .await {

                log::error!("server error: {:?}", err);
            }
        });
    }
}