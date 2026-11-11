mod sse;

use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};

use proto::telemetry_service_server::{TelemetryService as Telemetry, TelemetryServiceServer};
use proto::{TelemetryRequest, TelemetryResponse};

use rand::RngExt;

pub mod proto {
    tonic::include_proto!("telemetry");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("telemetry_descriptor");
}

pub struct TelemetryService {
    tx: broadcast::Sender<TelemetryResponse>,
}

#[tonic::async_trait]
impl Telemetry for TelemetryService {
    type TelemetryStream = ReceiverStream<Result<TelemetryResponse, Status>>;

    async fn telemetry(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<Self::TelemetryStream>, Status> {
        let id = request.into_inner().id;
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let mut bus = self.tx.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tx.closed() => break,
                    received = bus.recv() => match received {
                        Ok(message) => {
                            if id
                                .as_deref()
                                .is_some_and(|id| !id.is_empty() && message.id != id)
                            {
                                continue;
                            }

                            if tx.send(Ok(message)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("gRPC client lagged, skipped {n} messages");
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    },
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

async fn generate_device(
    device: String,
    start: [f64; 2],
    range: [f64; 4],
    tx: broadcast::Sender<TelemetryResponse>,
) {
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
    let mut seq = 0u64;

    let route_size: f64 = rand::rng().random_range(0.005..=0.02);

    let x0 = (start[0] - route_size).max(range[0]);
    let x1 = (start[0] + route_size).min(range[1]);
    let y0 = (start[1] - route_size).max(range[2]);
    let y1 = (start[1] + route_size).min(range[3]);

    let waypoints = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    let mut current_waypoint = rand::rng().random_range(0..=3);

    let prev = waypoints[(current_waypoint + 3) % 4];
    let t: f64 = rand::rng().random_range(0.0..1.0);
    let mut current = [
        prev[0] + (waypoints[current_waypoint][0] - prev[0]) * t,
        prev[1] + (waypoints[current_waypoint][1] - prev[1]) * t,
    ];

    let step: f64 = rand::rng().random_range(0.0003..=0.002);

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

    let start_point: [f64; 2] = [-1.297848, 50.676592];

    let half_size_meters: f64 = 5000.0;
    let meters_per_degree_latitude: f64 = 111320.0;
    let meters_per_degree_longitude: f64 =
        meters_per_degree_latitude * start_point[1].to_radians().cos();

    let degrees_longitude = half_size_meters / meters_per_degree_longitude;
    let degrees_latitude = half_size_meters / meters_per_degree_latitude;

    let mut rng = rand::rng();
    for n in 1..=20 {
        let i = n - 1;
        let col = (i % 5) as f64;
        let row = (i / 5) as f64;

        let scale_long: f64 = -1.0 + (col + rng.random_range(0.1..0.9)) * (2.0 / 5.0);
        let scale_lat: f64 = -1.0 + (row + rng.random_range(0.1..0.9)) * (2.0 / 4.0);

        let long: f64 = start_point[0] + degrees_longitude * scale_long;
        let lat: f64 = start_point[1] + degrees_latitude * scale_lat;

        let range: [f64; 4] = [
            start_point[0] - 0.22,
            start_point[0] + 0.22,
            start_point[1] - 0.14,
            start_point[1] + 0.14,
        ];

        let device = format!("SIM-{n}");
        tokio::spawn(generate_device(device, [long, lat], range, tx.clone()));
    }

    let mut rx = tx.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(message) => {
                    let _ = sse_tx.send(
                        serde_json::json!({
                            "id": message.id,
                            "sequence_number": message.sequence_number,
                            "latitude": message.latitude,
                            "longitude": message.longitude,
                            "altitude": message.altitude,
                        })
                        .to_string(),
                    );
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("SSE bridge lagged, skipped {n} messages");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let http = async {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:2199").await?;
        axum::serve(listener, app).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    let grpc = async {
        let address = "0.0.0.0:21990".parse()?;

        let reflection = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build_v1()?;

        Server::builder()
            .add_service(reflection)
            .add_service(TelemetryServiceServer::new(TelemetryService { tx }))
            .serve(address)
            .await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    tokio::try_join!(http, grpc)?;

    Ok(())
}
