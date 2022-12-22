use std::str::FromStr;

use axum::{Json, Router};
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::ws::{CloseFrame, Message};
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use futures::{SinkExt, StreamExt};
use hyper::{Body, Client, Uri};
use hyper::client::HttpConnector;
use tokio::{join, spawn};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::Message as TMessage;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tracing::trace;

use crate::info::GlobalInfo;
use crate::util::config::get_config;
use crate::util::string::EmptyExt;
use crate::web::v1::get_v1;

pub fn build_route(mut route: Router) -> Router {
	{
		let cfg = futures::executor::block_on(get_config());
		if !cfg.http.root_proxy.is_empty() {
			route = route
				.route("/", any(handler).with_state(Client::new()))
				.route("/*path", any(handler).with_state(Client::new()));
		}
	}
	route
		.route("/api", get(api_info))
		.nest("/api/v1", get_v1())
		.fallback(not_found)
}

async fn api_info() -> impl IntoResponse {
	Json(GlobalInfo::default())
}

async fn handler(ws: Option<WebSocketUpgrade>, State(client): State<Client<HttpConnector>>, mut req: Request<Body>) -> Response {
	if let Some(ws) = ws {
		let mut uri = {
			let cfg = get_config().await;
			format!("{}{}", cfg.http.root_proxy.as_ref().unwrap(), req.uri().path_and_query().map(|it| it.as_str()).unwrap())
		};

		uri.replace_range(..4, "ws");
		trace!("construct reverse websocket to {uri:?}");
		let uri = Uri::from_str(&uri).unwrap();
		let mut reqb = Request::builder()
			.uri(&uri)
			.method("GET")
			.header("Host", format!("{}:{}", uri.host().unwrap(), uri.port_u16().unwrap_or(80)))
			.header("Connection", "Upgrade")
			.header("Upgrade", "websocket")
			.header("Sec-WebSocket-Key", generate_key());
		let mut protocols: Vec<String> = Vec::new();
		for (k, v) in req.headers().iter() {
			let kstr = k.as_str();
			if kstr.starts_with("sec-websocket") {
				reqb = reqb.header(k, v);
			}
			if kstr.ends_with("protocol") {
				protocols.push(String::from_utf8_lossy(v.as_bytes()).into_owned())
			}
		}
		println!("{:?}", req.headers());
		println!("{:?}", reqb.headers_ref());
		let reqb = reqb
			.body(())
			.unwrap();

		if let Ok((origin, _)) = connect_async(reqb).await {
			trace!("connected to ws origin");
			return ws.protocols(protocols).on_upgrade(|ws| {
				trace!("upgraded current connection");
				async move {
					let (mut send, mut recv) = ws.split();
					let (mut osend, mut orecv) = origin.split();
					let forward = spawn(async move {
						while let Some(Ok(res)) = orecv.next().await {
							send.send(match res {
								TMessage::Text(s) => {
									Message::Text(s)
								}
								TMessage::Binary(s) => {
									Message::Binary(s)
								}
								TMessage::Ping(s) => {
									Message::Ping(s)
								}
								TMessage::Pong(s) => {
									Message::Pong(s)
								}
								TMessage::Close(s) => {
									Message::Close(s.map(|it| CloseFrame {
										code: Into::<u16>::into(it.code),
										reason: it.reason,
									}))
								}
								TMessage::Frame(_) => { unreachable!() }
							}).await?;
						}
						anyhow::Result::<()>::Ok(())
					});
					let backward = spawn(async move {
						while let Some(Ok(res)) = recv.next().await {
							osend.send(match res {
								Message::Text(s) => {
									TMessage::Text(s)
								}
								Message::Binary(s) => {
									TMessage::Binary(s)
								}
								Message::Ping(s) => {
									TMessage::Ping(s)
								}
								Message::Pong(s) => {
									TMessage::Pong(s)
								}
								Message::Close(s) => {
									TMessage::Close(s.map(|it| tokio_tungstenite::tungstenite::protocol::CloseFrame {
										code: CloseCode::from(it.code),
										reason: it.reason,
									}))
								}
							}).await?;
						}
						anyhow::Result::<()>::Ok(())
					});
					let _ = join!(forward,backward);
				}
			});
		};
		trace!("failed to create ws tunnel");
		// ws_origin.next().await.unwrap().unwrap().
		"".into_response()
	} else {
		let path = req.uri().path();
		let path_query = req
			.uri()
			.path_and_query()
			.map(|v| v.as_str())
			.unwrap_or(path);

		let uri = {
			let cfg = get_config().await;
			format!("{}{}", cfg.http.root_proxy.as_ref().unwrap(), path_query)
		};

		*req.uri_mut() = Uri::try_from(uri).unwrap();
		if let Ok(resp) = client.request(req).await {
			resp.into_response()
		} else {
			not_found().await.into_response()
		}
	}
}

async fn not_found() -> impl IntoResponse {
	(StatusCode::NOT_FOUND, r#"{"success":false,"message":"The resource you are looking for is unavailable"}"#)
}