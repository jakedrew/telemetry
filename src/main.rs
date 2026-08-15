mod sse;

use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

use proto::{TelemetryRequest, TelemetryResponse};
use proto::telemetry_service_server::{TelemetryService as Telemetry, TelemetryServiceServer};

pub mod proto {
    tonic::include_proto!("telemetry");
}

pub struct TelemetryService {
    tx: broadcast::Sender<TelemetryResponse>,
}

/* 
 * TODO:
 *  - Randomise location start points (within radius N of some point)
 *  - Move point within this area
 *  - Spawn between 5 - 50 devices for map
 */                

#[tonic::async_trait]
impl Telemetry for TelemetryService {
    type TelemetryStream = ReceiverStream<Result<TelemetryResponse, Status>>;

    async fn telemetry(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<Self::TelemetryStream>, Status> {
        let _req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let mut bus = self.tx.subscribe();

        tokio::spawn(async move {
            loop {
                match bus.recv().await {
                    Ok(message) => {
                        if tx.send(Ok(message)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_n)) => { continue; }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

async fn generate_device(device: String, tx: broadcast::Sender<TelemetryResponse>) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut seq = 0u64;

    loop {
        tick.tick().await;

        let reply = TelemetryResponse {
            id: device.clone(),
            sequence_number: seq,
            latitude: 3.0,
            longitude: 4.0,
            altitude: 5.0,
        };

        let _ = tx.send(reply);
        seq += 1;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (app, sse_tx) = sse::router();
    let (tx, _) = broadcast::channel::<TelemetryResponse>(256);

    for n in 1..=5 {
        let device = format!("SIM-{n}");
        tokio::spawn(generate_device(device, tx.clone()));
    }

    let mut rx = tx.subscribe();
    tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {

            let _ = sse_tx.send(
                serde_json::json!({
                    "id": message.id,
                    "sequence_number": message.sequence_number,
                    "latitude": message.latitude,
                    "longitude": message.longitude,
                    "altitude": message.altitude,
                }).to_string(),
            );
        }
    });

    tokio::spawn(async {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    let address = "0.0.0.0:50051".parse()?;
    Server::builder()
        .add_service(TelemetryServiceServer::new(TelemetryService { tx }))
        .serve(address)
        .await?;

    Ok(())
}
