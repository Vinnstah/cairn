# Cairn

## What is it?

Cairn is a Rust service for querying and replaying autonomous vehicle sensor data. It ingests multi-sensor driving data from the [NVIDIA PhysicalAI Autonomous Vehicles dataset](https://huggingface.co/datasets/nvidia/PhysicalAI-Autonomous-Vehicles), exposes it over an HTTP API, contains a frontend to select filters and query the backend and streams the results live into [Rerun](https://rerun.io/) for real-time visualization.

Given a set of driving conditions, Cairn finds matching 20-second clips and replays their [ego motion trajectories](https://en.wikipedia.org/wiki/Visual_odometry), camera footage, and [LiDAR point clouds](https://en.wikipedia.org/wiki/Point_cloud) in a synchronized 3D viewer.

---

## Why?
The primary objective of Cairn is **scenario mining**. Training multimodal machine learning models for autonomous vehicles requires large amounts of data. The amount of data is not the primary requirement but it also needs to be diverse.
If the scenarios you train the model on are not diverse enough it can lead to [overfitting](https://en.wikipedia.org/wiki/Overfittinghttps://en.wikipedia.org/wiki/Overfitting). You woud have a model that is highly precise on the data it has been trained on, but if you introduce new scenarios the model would have insufficient accuracy.

Cairn explores a solution to this: a app to query local datasets to find scenarios containing desirable data. For example: *find all clips where the vehicle was decelerating above 2.5 m/s² that contains a car*. This is done using a compiled, async Rust service with [DataFusion](https://datafusion.apache.org/) as an embedded columnar query engine. This means:

- **Fast predicate pushdown** over large Parquet datasets. We perform the filtering on the source [Parquet-files](https://parquet.apache.org/). [source](https://pola.rs/posts/predicate-pushdown-query-optimizer/)
- **Concurrent queries** 
- **Live streaming** of query results directly into a 3D visualization tool for validation

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

# Run the service
cargo run ./backend

# After the backend is up and have registered the files
cargo run ./frontend
```

**NOTE** Further adjustments and improvements will be made to enhance the startup procedure. 

### Querying files without the frontend

```bash
# Search clips matching driving conditions
curl "http://localhost:3000/clips/search?min_speed=15.0&min_decel=2.5"
```

```bash
# Replay clips matching driving conditions
curl "http://localhost:3000/clips/replay?min_speed=15.0&min_decel=2.5"
```

Each request queries the Parquet files via DataFusion SQL, finds matching clip UUIDs, then streams their ego motion trajectory, Draco-decoded LiDAR point clouds into the running Rerun viewer.

---

## Architecture
The app is split into 3 different crate: *backend*, *frontend* and *shared*.

*shared*: contains types shared between frontend and backend
*backend*: follows a ports-and-adapters (hexagonal) architecture. The core domain has no dependencies on infrastructure — DataFusion, Rerun, and Axum are all behind interfaces.
*frontend*: a GUI-app created with *eframe* and *egui*

### Dataset layout expected on disk

```
./data/nvidia_physical_dataset/
├── egomotion.chunk_0000/
│   └── <clip_uuid>.egomotion.parquet         # pose, velocity, acceleration ~100Hz
├── camera_front_wide_120fov.chunk_0000/
│   ├── <clip_uuid>.camera_front_wide_120fov.mp4
│   └── <clip_uuid>.camera_front_wide_120fov.timestamps.parquet
├── lidar.chunk_0000/
│   └── <clip_uuid>.lidar_top_360fov.parquet  # Draco-encoded point clouds ~200 spins/clip
├── obstacle.offline.chunk_0000/
│   └── <clip_uuid>.obstacle.offline.parquet  # Obstacles and bounding boxes
└── metadata/
    ├── data_collection.parquet               # country, month, hour_of_day per clip
    └── feature_presence.parquet             # sensor availability per clip
```

### Notable implementation details

**Clip ID injection** — The NVIDIA dataset stores clip UUID only in filenames, not as a column inside the Parquet files. All chunk folders are registered by iterating the directory, extracting the UUID prefix from each filename, and building a `UNION ALL` SQL view that adds `clip_id` as a literal column. This makes cross-table joins possible without modifying the raw data.

**Draco decoding** — LiDAR point clouds are Google Draco-compressed binary blobs stored as `BinaryView` columns. They are decoded in Rust via `draco-rs` (C++ FFI, requires cmake). The `PointClouds` newtype wraps `Vec<PointCloud>` to implement `TryFrom<RecordBatch>` without violating the orphan rule.

**Arrow version isolation** — Rerun and DataFusion both re-export Arrow but may pin different versions. All `RecordBatch` processing uses `datafusion::arrow` types exclusively. Conversion to Rerun types (`Points3D`, `Transform3D`) happens only at the `SceneLogger` adapter boundary.