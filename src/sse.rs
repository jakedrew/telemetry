use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::Router;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

pub type Tx = tokio::sync::broadcast::Sender<String>;

pub fn router() -> (Router, Tx) {
    let (tx, _rx) = tokio::sync::broadcast::channel::<String>(256);

    let router = Router::new()
        .route("/stream", get(stream_handler))
        .layer(CorsLayer::permissive())
        .with_state(tx.clone());

    return (router, tx);
}

async fn stream_handler(State(tx): State<Tx>,) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = tx.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|message| {
        match message {
            Ok(json) => Some(Ok(Event::default().data(json))),
            Err(_) => None,
        }
    });

    return Sse::new(stream).keep_alive(KeepAlive::default());
}