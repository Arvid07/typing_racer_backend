use socketioxide::SocketIo;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use crate::states::app_state::SharedAppState;

pub mod states;
pub mod text;
mod socket_events;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;

    let app_state = SharedAppState::default();
    let (layer, io) = SocketIo::builder().with_state(app_state).build_layer();
    
    io.ns("/", socket_events::handle_websocket_connection);

    let app = axum::Router::new()
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(layer)
        );

    info!("Starting Server");

    axum::Server::bind(&"127.0.0.1:3001".parse().unwrap())
        .serve(app.into_make_service())
        .await?;

    Ok(())
}