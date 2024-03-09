use gui::Item;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use types::ClientEvent;
use ui_client::create_ui_client;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;

pub mod gui;
mod edit_distance;
pub mod types;
mod ui_client;
mod diff;

pub const CLIENT_ID: AtomicU64 = AtomicU64::new(0);

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

const index_js_bytes: &[u8] = include_bytes!("../../dist/index.js");

struct Ctx {

}

async fn handle_req(mut req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if hyper_tungstenite::is_upgrade_request(&req) {
        if req.uri().path() == "/ws" {
            let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None).unwrap();
            let id = CLIENT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as usize;
            create_ui_client(id, websocket);
            return Ok(response);
        }
    }

    match req.uri().path() {
        "/index.js" => {
            Ok(Response::new(Full::new(Bytes::from(index_js_bytes))))
        },
        _ => {
            Ok(serve_index())
        }
    }
}

async fn server() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 4477));
    let listener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(socket);
        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_req))
                .await {

                log::error!("server error: {:?}", err);
            }
        });
    }
}

pub struct Wgui {
    pub events_rx: mpsc::UnboundedReceiver<ClientEvent>
}

impl Wgui {
    pub fn new() -> Self {
        let (events_tx, events_rx) = mpsc::unbounded_channel();

        Self {
            events_rx
        }
    }

    pub async fn next(&mut self) -> Option<ClientEvent> {
        self.events_rx.recv().await
    }

    pub fn render(&self, client_id: usize, item: Item) {
        println!("render {:?}", item);
    }
}