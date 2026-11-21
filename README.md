# Telemetry: Streaming gRPC/SSE to Map (Rust/Tokio)
2026-11-10
<!-- description: A Rust/Tokio service streaming live telemetry over gRPC and Server-Sent Events onto a browser map. Live demo and source on GitHub. -->
<!-- image: screenshot.png | Telemetry Map Screenshot -->

I wanted a small project to practise streaming data in Rust, and also to learn Protobufs. I decided to build 20 simulated drone-like devices which report their position twice a second, and one Rust server that streams this in two formats: gRPC (tonic), and Server Sent Events (SSE because there is no native gRPC for browsers). The map below is a live demo of this:

<div class="embed">
    <iframe src="https://telemetry.jakedrw.com/?embed=1" frameborder="0" loading="lazy"></iframe>
</div>

## Try it for yourself:

The map above is an iframe of the [Live map](https://telemetry.jakedrw.com/), so on the full site you can easily move around and zoom. It's a docker container built from [Source (GitHub)](https://github.com/jakedrew/telemetry).

In your terminal using a gRPC client (I chose [grpcurl](https://github.com/fullstorydev/grpcurl)) run:

```bash
grpcurl -d '{"id": "SIM-1"}' telemetry.jakedrw.com:443 telemetry.TelemetryService/Telemetry
```

and you'll see a stream of messages like this:

```json
{
  "id": "SIM-1",
  "sequenceNumber": "2373697",
  "latitude": 50.63460873872809,
  "longitude": -1.3539654600872957,
  "altitude": 5
}
{
  "id": "SIM-1",
  "sequenceNumber": "2373698",
  "latitude": 50.63460873872809,
  "longitude": -1.3529954276367737,
  "altitude": 5
}
{
  "id": "SIM-1",
  "sequenceNumber": "2373699",
  "latitude": 50.635641801053275,
  "longitude": -1.3529954276367737,
  "altitude": 5
}
{
  "id": "SIM-1",
  "sequenceNumber": "2373700",
  "latitude": 50.63667486337846,
  "longitude": -1.3529954276367737,
  "altitude": 5
}
```

Leave out the id (or send it empty) and you get every device.

## How it works

```mermaid
flowchart TB
    devices["20 simulated devices<br/>One tokio task each<br/>Updates every 500 ms"]
    devices --> bus["Broadcast channel<br/>Buffer 256"]

    subgraph ssePath["SSE (axum)"]
        bridge["Bridge task<br/>Converts to JSON once"]
        bridge --> bus2["Broadcast channel<br/>Buffer 256"]
        bus2 --> stream["/stream<br/>Subscription per browser"]
        stream --> map["OpenLayers map<br/>Via EventSource"]
    end

    subgraph grpcPath["gRPC (tonic)"]
        grpcTask["Task per client<br/>Filters by id if given"]
        grpcTask --> mpsc["mpsc channel<br/>Buffer 16"]
        mpsc --> grpcStream["Response stream"]
        grpcStream --> grpcurl["grpcurl"]
    end

    bus --> grpcTask
    bus --> bridge

    style ssePath fill:#e0f2fe,stroke:#0284c7,stroke-width:2px,color:#0284c7
    style grpcPath fill:#d5e1ff,stroke:#0055cc,stroke-width:2px,color:#0055cc
```

Each device is its own tokio task - they all send into one `tokio::sync::broadcast` channel (buffer of 256). Anything that wants the data just subscribes to this channel.

**For gRPC**: every client gets its own subscription plus a small `mpsc` channel (16) that feeds the response stream. If you pass an id then only that device is forwarded, otherwise you get all of them. A slow client only blocks its own task, and when it disconnects the task ends.

**For SSE**: I have one bridge task that turns each message into JSON once and puts it on a second broadcast channel (also 256), then each browser subscribes to that with `BroadcastStream`. As browsers can't do gRPC directly, I send it out as SSE instead, which works with `EventSource` and also reconnects on its own.

I also turned on server reflection (where a client can discover the services/methods), so grpcurl can list and call the service without you needing the `.proto` definition file.

The message itself is pretty simple:

[Github: proto/telemetry.proto](https://github.com/jakedrew/telemetry/blob/d4b210a44ca1807c16cbc232a637e872aa5e8ad7/proto/telemetry.proto)
```proto
syntax = "proto3";
package telemetry;

message TelemetryResponse {
  string id = 1;
  uint64 sequence_number = 2;
  double latitude = 3;
  double longitude = 4;
  double altitude = 5;
}

message TelemetryRequest {
  optional string id = 1;
}

service TelemetryService {
  rpc Telemetry (TelemetryRequest) returns (stream TelemetryResponse);
}
```

Each message is about 40 bytes as protobuf and about 120 as JSON. So 20 devices at 2 Hz is 40 messages a second, roughly 5 KB/s per browser.

If anything gets more than 256 messages behind, tokio gives it a `Lagged` error, and I just skip what was missed and carry on from the latest. With gRPC you'll see a jump in `sequence_number`, and on the map the device jumps forward.

## Simulated Devices

These devices are centred around the Isle of Wight, mainly because it's a recognisable location and at this zoom the water made the embedded map look more interesting than centering over land when embedded in the iframe.

The devices start in a 10 km square split into a 5×4 grid, and each device's route is centred in its own cell so they don't all bunch up. Each one then drives laps around a box with a random size (0.01–0.04 degrees across), random speed (0.0003–0.002 degrees per tick), and ticking every 500 ms. The boxes are square in degrees not metres, so on the map they end up taller than they are wide.

## Hosting

The container runs the Rust binary and a Caddy serving the map from `/srv`. My [jakedrw.com](https://jakedrw.com) Caddy sits in front and does TLS and routing through the docker network. gRPC (and reflection) goes to tonic, `/stream` goes to axum, and everything else goes to the internal Caddy.
