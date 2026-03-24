# Cairn

## Disclaimer
This project is currently prone to SQL-injection via the query params. Since the data is not stored in a persisted database, the tables are created on startup, 
this is a calculated risk that is acceptable.

## Etymology
Cairn - a trailmarker. Marks the way.

## Architecture
This project uses Hexagonal architecture.

### Services
    - DataFusion service (filter push-down)
    - HTTP server (Axum)
    - Visualizer (Rerun)

# Requirements

Rerun installed via:
```cargo install --force rerun-cli@0.30.2```

```
nvidia/PhysicalAI-Autonomous-Vehicles dataset downloaded locally
```

# Examples
```
TBA
```

## TODO
Add multimodal data to the Replay. Currently we only support Lidar and Video to some extent, but not combined. Support should be added for radar as well as the metadata.