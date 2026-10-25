# telemetry

## Testing gRPC

This can be tested with:

```bash 
grpcurl -plaintext -proto proto/telemetry.proto \
-d '{"id": "SIM-01"}' \
'localhost:50051' telemetry.TelemetryService/Telemetry
```

Returning:

 ```json
 {
  "id": "SIM-01",
  "latitude": 3,
  "longitude": 4,
  "altitude": 5
}
{
  "id": "SIM-01",
  "sequenceNumber": "1",
  "latitude": 3,
  "longitude": 4,
  "altitude": 5
}
{
  "id": "SIM-01",
  "sequenceNumber": "2",
  "latitude": 3,
  "longitude": 4,
  "altitude": 5
}
{
  "id": "SIM-01",
  "sequenceNumber": "3",
  "latitude": 3,
  "longitude": 4,
  "altitude": 5
}
{
  "id": "SIM-01",
  "sequenceNumber": "4",
  "latitude": 3,
  "longitude": 4,
  "altitude": 5
}
```