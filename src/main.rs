mod sse;

use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

use proto::{TelemetryRequest, TelemetryResponse};
use proto::telemetry_service_server::{TelemetryService as Telemetry, TelemetryServiceServer};

use rand::RngExt;

pub mod proto {
    tonic::include_proto!("telemetry");
}

pub struct TelemetryService {
    tx: broadcast::Sender<TelemetryResponse>,
}

/* 
 * TODO:
 *  [X] Randomise location start points (within radius N of some point)
 *  [X] Move point within this area
 *  [X] Spawn between 5 - 50 devices for map
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

async fn generate_device(device: String, start: [f64; 2], range: [f64; 4], tx: broadcast::Sender<TelemetryResponse>) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut seq = 0u64;

    let route_size = 0.01;

    let x0 = (start[0] - route_size).max(range[0]);
    let x1 = (start[0] + route_size).min(range[1]);
    let y0 = (start[1] - route_size).max(range[2]);
    let y1 = (start[1] + route_size).min(range[3]);

    let waypoints = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    let mut current_waypoint = 0;
    let mut current = waypoints[0];

    let step = 0.0002;

    loop {
        tick.tick().await;

        let reply = TelemetryResponse {
            id: device.clone(),
            sequence_number: seq,
            latitude: current[1],
            longitude: current[0],
            altitude: 5.0,
        };

        let _ = tx.send(reply);

        let dx = waypoints[current_waypoint][0] - current[0];
        let dy = waypoints[current_waypoint][1] - current[1];

        let dist = dx.hypot(dy);

        if dist < step {
            current = waypoints[current_waypoint];
            current_waypoint = (current_waypoint + 1) % 4;
        } else {
            current[0] += dx / dist * step;
            current[1] += dy / dist * step;
        }

        seq += 1;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (app, sse_tx) = sse::router();
    
    let (tx, _) = broadcast::channel::<TelemetryResponse>(256);

    let mut rng = rand::rng();
    for n in 1..=25 {
        // long, lat to match the openlayer
        let start_point = [-1.297848, 50.676592];

        let scale_long: f64 = rng.random_range(-1.0..=1.0);
        let scale_lat: f64 = rng.random_range(-1.0..=1.0);

        let long: f64 = start_point[0] + (0.22 * scale_long);
        let lat: f64 = start_point[1] + (0.14 * scale_lat);

        let range: [f64; 4] = [
            start_point[0] - 0.22, 
            start_point[0] + 0.22,
            start_point[1] - 0.14, 
            start_point[1] + 0.14
        ];

        let device = format!("SIM-{n}");
        tokio::spawn(generate_device(device, [long, lat], range, tx.clone()));
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
