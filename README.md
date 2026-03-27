# Cairn

## What is it?

Cairn is a Rust service for querying and replaying autonomous vehicle sensor data. It ingests multi-sensor driving data from the [NVIDIA PhysicalAI Autonomous Vehicles dataset](https://huggingface.co/datasets/nvidia/PhysicalAI-Autonomous-Vehicles), exposes it over an HTTP API, and streams the results live into [Rerun](https://rerun.io/) for real-time visualization.

Given a set of driving conditions, Cairn finds matching 20-second clips and replays their ego motion trajectories, camera footage, and LiDAR point clouds in a synchronized 3D viewer — scrubable on a timeline.

---

## Why?

Most tooling for working with autonomous vehicle datasets is Python-first, batch-oriented, and not designed for interactive or low-latency access. Running SQL over hundreds of gigabytes of Parquet in Python works, but it's slow and hard to serve concurrently.

Cairn explores a different approach: a compiled, async Rust service using [DataFusion](https://datafusion.apache.org/) as an embedded columnar query engine. This means:

- **Fast predicate pushdown** over large Parquet datasets without a Spark cluster
- **Concurrent queries** without the overhead of a Python runtime
- **Live streaming** of query results directly into a 3D visualization tool

The primary use case is **scenario mining** — finding edge cases in large driving datasets by querying ego motion and sensor metadata. For example: *find all clips where the vehicle was decelerating above 2.5 m/s²*. The matching clips stream directly into Rerun as a synchronized 3D replay of trajectory, point clouds, and camera footage.

---

## How?

### Prerequisites

- Rust 1.88+
- [cmake](https://cmake.org/) — required for Draco C++ point cloud bindings: `brew install cmake`
- A HuggingFace account with the [NVIDIA PhysicalAI-AV license accepted](https://huggingface.co/datasets/nvidia/PhysicalAI-Autonomous-Vehicles)
- [Rerun viewer](https://rerun.io/): `cargo install rerun-cli --locked`

### Setup

```bash
# Clone the repo
git clone https://github.com/Vinnstah/cairn
cd cairn

# Download a real subset from HuggingFace
HF_TOKEN=hf_... python scripts/download.py --num-clips 10

# Start the Rerun viewer
rerun &

# Run the service
cargo run
```

### Querying

```bash
# Replay clips matching driving conditions
curl "http://localhost:3000/clips/search?min_speed=15.0&min_decel=2.5"
```

Each request queries the Parquet files via DataFusion SQL, finds matching clip UUIDs, then streams their ego motion trajectory, Draco-decoded LiDAR point clouds, and camera footage into the running Rerun viewer.

---

## Architecture

Cairn follows a ports-and-adapters (hexagonal) architecture. The core domain has no dependencies on infrastructure — DataFusion, Rerun, and Axum are all behind interfaces.

```
┌──────────────────────────────────────────────────────────────┐
│                        HTTP Layer                            │
│                     (Axum, port 3000)                        │
│                      /clips/search                           │
│                      /clips/replay                           │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                    Inbound Ports                              │
│                                                               │
│   DataQuery (trait)            Replay (trait)                 │
│   fetch_ego_motion             replay_clips                   │
│   fetch_clips_with_params                                     │
│   fetch_point_clouds                                          │
│                                                              │
└──────────────┬────────────────────────┬─────────────────────┘
               │                        │
               ▼                        ▼
┌──────────────────────┐  ┌─────────────────────────────────────┐
│   DataQueryService   │  │           ReplayService              │
│                      │  │                                       │
│  Implements          │  │  Implements Replay (inbound)          │
│  DataQuery (inbound) │  │  Orchestrates DataQuery + SceneLogger │
│  Delegates to        │  │  Fetches clips → fetches sensor data  │
│  DataStore (outbound)│  │  → streams to SceneLogger             │
└──────────┬───────────┘  └──────────────────┬──────────────────┘
           │                                 │
           ▼                                 ▼
┌──────────────────────┐  ┌─────────────────────────────────────┐
│   Outbound Port      │  │         Outbound Port                │
│   DataStore (trait)  │  │         SceneLogger (trait)          │
│                      │  │                                      │
│                      │  │  replay_ego_motion(Vec<EgoMotion>)   │
│                      │  │  replay_point_clouds(Vec<PointCloud>)│
│  query_clips_with_   │  │  visualize_video(AssetVideo)         │
│    params            │  │                                      │
│  query_point_clouds  │  │                                      │
│  query_ego_motion    │  │                                      │
│  register_tables     │  │                                      │
└──────────┬───────────┘  └──────────────────┬──────────────────┘
           │                                 │
           ▼                                 ▼
┌──────────────────────┐  ┌─────────────────────────────────────┐
│    SessionContext     │  │           RecordingStream            │
│    (DataFusion)       │  │           (Rerun)                    │
│                       │  │                                       │
│  Parquet tables       │  │  Logs Transform3D per ego sample     │
│  registered as SQL    │  │  Logs LineStrips3D trajectory        │
│  views with clip_id   │  │  Logs Points3D per LiDAR spin        │
│  injected from        │  │  Logs AssetVideo for camera clips    │
│  filenames            │  │  Sets timeline per spin/timestamp    │
│                       │  │                                       │
│  Decodes Draco point  │  │                                       │
│  clouds via draco-rs  │  │                                       │
└──────────────────────┘  └─────────────────────────────────────┘
```

### Key components

**`core/domain/model`** — Plain Rust structs with no infrastructure dependencies: `ClipSearchParams`, `PointCloud`, `PointClouds`, `EgoMotion`, `DataError`.

**`core/ports/inbound/data_query`** — `DataQuery` trait: fetches clips and sensor data. Called by HTTP handlers.

**`core/ports/inbound/replay`** — `Replay` trait: orchestrates a full clip replay. Called by HTTP handlers.

**`core/ports/outbound/data_store`** — `DataStore` trait: all DataFusion interactions. Returns domain types, no Arrow or Parquet types leak out.

**`core/ports/outbound/scene_logger`** — `SceneLogger` trait: streams sensor data into a visualization tool. Takes domain types (`EgoMotion`, `PointCloud`), not Rerun types.

**`core/services/data_query_service`** — Implements `DataQuery` by delegating to `DataStore`. Cross-cutting concerns (logging, metrics) live here.

**`core/services/replay_service`** — Implements `Replay`. Fetches clip IDs via `DataQuery`, then for each clip fetches point clouds and ego motion and hands them to `SceneLogger`.

**`adapters/querier`** — `SessionContext` implements `DataStore`. Registers all Parquet chunk folders as DataFusion SQL views, injecting `clip_id` from filenames via a `UNION ALL` view pattern. Decodes Draco-compressed LiDAR via `draco-rs` into `Vec<PointCloud>`. Converts `RecordBatch` columns to `EgoMotion` via `as_primitive`.

**`adapters/rerun`** — `RecordingStream` implements `SceneLogger`. Logs ego motion as `Transform3D` + `LineStrips3D` trajectory, LiDAR spins as `Points3D` on a `spin` timeline, and camera footage as `AssetVideo` with auto-detected frame timestamps.

**`adapters/http`** — Axum router with typed `Query` extractors. `AppState` holds `Arc<dyn DataQuery>` and `Arc<dyn Replay>`.

### Data flow for a `/clips/replay` request

```
1. Axum deserializes ClipSearchParams { min_speed, min_decel } from query string
2. clips_replay_handler calls state.replayer.replay_clips(params)
3. ReplayService calls DataQuery.fetch_clips_with_params(params)
4. DataQueryService delegates to DataStore.query_clips_with_params(params)
5. SessionContext executes a GROUP BY / HAVING SQL query over ego_motion Parquet
6. Matching clip UUIDs returned to ReplayService
7. For each clip_id:
   a. DataQuery.fetch_point_clouds(clip_id, num_spins) → Vec<PointCloud>
      - SQL query against lidar table (registered from lidar.chunk_0000/)
      - Each row is one Draco-encoded spin, decoded via draco-rs
      - Converted via PointClouds newtype (orphan rule workaround)
   b. DataQuery.fetch_ego_motion(clip_id) → Vec<EgoMotion>
      - SQL query against ego_motion table
      - RecordBatch columns extracted via as_primitive::<Float64Type>()
   c. SceneLogger.replay_point_clouds(point_clouds)
      - Each PointCloud logged as Points3D with spin timeline index
   d. SceneLogger.replay_ego_motion(ego_motion)
      - Each sample logged as Transform3D
      - Full path logged as LineStrips3D
8. HTTP handler returns 200 OK; visualization appears live in Rerun viewer
```

### Dataset layout expected on disk

```
data/nvidia_physical_dataset/
├── egomotion.chunk_0000/
│   └── <clip_uuid>.egomotion.parquet         # pose, velocity, acceleration ~100Hz
├── camera_front_wide_120fov.chunk_0000/
│   ├── <clip_uuid>.camera_front_wide_120fov.mp4
│   └── <clip_uuid>.camera_front_wide_120fov.timestamps.parquet
├── lidar.chunk_0000/
│   └── <clip_uuid>.lidar_top_360fov.parquet  # Draco-encoded point clouds ~200 spins/clip
└── metadata/
    ├── data_collection.parquet               # country, month, hour_of_day per clip
    └── feature_presence.parquet             # sensor availability per clip
```

### Notable implementation details

**Clip ID injection** — The NVIDIA dataset stores clip UUID only in filenames, not as a column inside the Parquet files. All chunk folders are registered by iterating the directory, extracting the UUID prefix from each filename, and building a `UNION ALL` SQL view that adds `clip_id` as a literal column. This makes cross-table joins possible without modifying the raw data.

**Draco decoding** — LiDAR point clouds are Google Draco-compressed binary blobs stored as `BinaryView` columns. They are decoded in Rust via `draco-rs` (C++ FFI, requires cmake). The `PointClouds` newtype wraps `Vec<PointCloud>` to implement `TryFrom<RecordBatch>` without violating the orphan rule.

**Arrow version isolation** — Rerun and DataFusion both re-export Arrow but may pin different versions. All `RecordBatch` processing uses `datafusion::arrow` types exclusively. Conversion to Rerun types (`Points3D`, `Transform3D`) happens only at the `SceneLogger` adapter boundary.