use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

use proto::{TelemetryRequest, TelemetryResponse};
use proto::telemetry_service_server::{TelemetryService as Telemetry, TelemetryServiceServer};

pub mod proto {
    tonic::include_proto!("telemetry");
}

#[derive(Debug, Default)]
pub struct TelemetryService {}

#[tonic::async_trait]
impl Telemetry for TelemetryService {
    type TelemetryStream = ReceiverStream<Result<TelemetryResponse, Status>>;

    async fn telemetry(
        &self,
        request: Request<TelemetryRequest>,
    ) -> Result<Response<Self::TelemetryStream>, Status> {
        let _req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        tokio::spawn(async move {
            let mut seq = 0u64;
            loop {
                /* 
                 * TODO:
                 *  - Randomise location start points (within radius N of some point)
                 *  - Move point within this area
                 *  - Spawn between 5 - 50 devices for map
                 */                

                let reply: TelemetryResponse = TelemetryResponse {
                    id: String::from("SIM-01"),
                    sequence_number: seq,
                    latitude: 3.0,
                    longitude: 4.0,
                    altitude: 5.0,
                };

                if tx.send(Ok(reply)).await.is_err() {
                    break;
                }

                seq += 1;

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    Server::builder()
        .add_service(TelemetryServiceServer::new(TelemetryService::default()))
        .serve(addr)
        .await?;

    Ok(())
}
