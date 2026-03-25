# Cairn

## What is it?

Cairn is a Rust service for querying and replaying autonomous vehicle sensor data. It ingests multi-sensor driving data from the [NVIDIA PhysicalAI Autonomous Vehicles dataset](https://huggingface.co/datasets/nvidia/PhysicalAI-Autonomous-Vehicles), exposes it over an HTTP API, and streams the results live into [Rerun](https://rerun.io/) for real-time visualization.

Given driving conditions, Cairn finds matching clips and replays their ego motion trajectories, camera footage, and LiDAR point clouds in a synchronized 3D viewer.

---

## Why?

Most tooling for working with autonomous vehicle datasets is Python-first, batch-oriented, and not designed for interactive or low-latency access. Running SQL over hundreds of gigabytes of Parquet in Python works, but it's slow and hard to serve concurrently.

Cairn explores a different approach: a compiled, async Rust service using [DataFusion](https://datafusion.apache.org/) as an embedded columnar query engine. This means:

- **Fast predicate pushdown** over large Parquet datasets without a Spark cluster
- **Concurrent queries** without the overhead of a Python runtime
- **Live streaming** of query results directly into a visualization tool

The primary use case is **scenario mining** — finding edge cases in large driving datasets by querying ego motion and sensor metadata. For example: *find all clips where the vehicle was decelerating above 2.5 m/s² while the front-wide camera was active*. The result streams directly into Rerun as a 3D replay.

---

## How?

### Prerequisites

- Rust 1.88+
- [cmake](https://cmake.org/) (required for Draco C++ bindings: `brew install cmake`)
- A HuggingFace account with the [NVIDIA PhysicalAI-AV license accepted](https://huggingface.co/datasets/nvidia/PhysicalAI-Autonomous-Vehicles)
- [Rerun viewer](https://rerun.io/): `cargo install rerun-cli --locked`

### Setup

```bash
# Clone the repo
git clone https://github.com/yourhandle/cairn
cd cairn

# Download a subset of the dataset
HF_TOKEN=hf_... python scripts/download.py --num-clips 10

# Run the service
cargo run
```

### Querying

```bash
# Find clips by driving conditions
curl "http://localhost:3000/clips/search?min_speed=15.0&min_decel=2.5"
```

Each request queries the Parquet files via DataFusion SQL, resolves the matching camera and LiDAR files by clip UUID, and streams the results into the running Rerun viewer.

### Synthetic data

If you don't have dataset access yet, generate synthetic data in the same schema:

```bash
python scripts/generate_synthetic_av_data.py --num-clips 20
```

This produces ego motion Parquet files, camera MP4 stubs, and metadata tables that the service treats identically to the real dataset.

---

## Architecture

Cairn follows a ports-and-adapters (hexagonal) architecture. The core domain has no dependencies on infrastructure — DataFusion, Rerun, and Axum are all behind interfaces.

```
┌─────────────────────────────────────────────────────┐
│                     HTTP Layer                       │
│                  (Axum, port 3000)                   │
│                    /clips/search                     │
│                    /clips/replay                     │
└────────────────────────┬────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                  Inbound Port                        │
│               DataQuery (trait)                      │
│           fetch_clips_with_params                    │
│               fetch_point_cloud                      │
└────────────────────────┬────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│              DataQueryService                        │
│         (orchestrates outbound ports)                │
└──────────────┬──────────────────────┬───────────────┘
               │                      │
               ▼                      ▼
┌──────────────────────┐  ┌───────────────────────────┐
│   Outbound Port      │  │      Outbound Port         │
│   DataStore (trait)  │  │    VideoReplay (trait)     │
└──────────┬───────────┘  └────────────┬──────────────┘
           │                           │
           ▼                           ▼
┌──────────────────────┐  ┌───────────────────────────┐
│  SessionContext       │  │       RerunAdapter         │
│  (DataFusion)         │  │    (RecordingStream)       │
│                       │  │                            │
│  ego_motion           │  │  log ego trajectory        │
│  camera_timestamps    │  │  log camera video          │
│  lidar                │  │  log lidar point clouds    │
│  data_collection      │  │                            │
│  feature_presence     │  │                            │
└──────────────────────┘  └───────────────────────────┘
```

### Key components

**`core/domain/model`** — Plain Rust structs with no infrastructure dependencies: `ClipSearchParams`, `PointCloud`, `DataError`.

**`core/ports/inbound`** — `DataQuery` trait: the interface the HTTP layer calls into. Defined in terms of domain types only.

**`core/ports/outbound`** — `DataStore` and `VideoReplay` traits: how the service reaches out to DataFusion and Rerun respectively.

**`core/ports/data_query_service`** — `DataQueryService` implements `DataQuery` by delegating to `DataStore` and `VideoReplay`. This is where cross-cutting concerns like logging and metrics would live.

**`adapters/querier`** — `SessionContext` implements `DataStore`. Registers Parquet files as DataFusion views with `clip_id` injected from filenames, builds SQL queries from search params, and decodes Draco-compressed LiDAR point clouds via `draco-rs`.

**`adapters/rerun`** — `RecordingStream` implements `VideoReplay`. Logs ego motion poses, `AssetVideo` for camera clips, and `Points3D` for LiDAR spins to the Rerun timeline.

**`adapters/http`** — Axum router and handlers. Extracts typed query params, calls through `DataQuery`, returns JSON or streams results to Rerun.

### Data flow for a `/clips/replay` request

```
1. HTTP handler deserializes ClipSearchParams from query string
2. DataQueryService.fetch_point_clouds() called
3. DataStore builds a HAVING query over ego_motion grouped by clip_id
4. DataFusion executes SQL over Parquet, returns point_clouds to replay
5. For each clip_id:
   b. LiDAR parquet queried, Draco spins decoded into Vec<PointCloud>
6. VideoReplay logs trajectory, video, and point clouds to Rerun
   with spin_index on the timeline for scrubbing
```

### Dataset layout expected on disk

```
data/nvidia_physical_dataset/
├── egomotion.chunk_0000/
│   └── <clip_uuid>.egomotion.parquet        # pose, velocity, acceleration at ~100Hz
├── camera_front_wide_120fov.chunk_0000/
│   ├── <clip_uuid>.camera_front_wide_120fov.mp4
│   └── <clip_uuid>.camera_front_wide_120fov.timestamps.parquet
├── lidar.chunk_0000/
│   └── <clip_uuid>.lidar_top_360fov.parquet # Draco-encoded point clouds, ~200 spins/clip
└── metadata/
    ├── data_collection.parquet              # country, month, hour_of_day per clip
    └── feature_presence.parquet            # which sensors are present per clip
```