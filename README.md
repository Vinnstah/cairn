# Cairn
 
Cairn is a scenario mining tool for autonomous vehicle data built in Rust. It lets engineers query a large multi-sensor driving dataset using human-readable conditions and instantly replay the matching clips as a synchronized 3D visualization of ego trajectory, LiDAR point clouds, and labeled bounding boxes.
 
---
 
## What is it?
 
### The setup
 
A developer runs the Cairn backend once against a local copy of the [NVIDIA PhysicalAI Autonomous Vehicles dataset](https://huggingface.co/datasets/nvidia/PhysicalAI-Autonomous-Vehicles). On startup, the backend registers all sensor data — ego motion, LiDAR, camera timestamps, and obstacle detections — as queryable SQL tables backed by [Parquet](https://parquet.apache.org) files on disk. No data is copied or transformed; the raw dataset files are indexed in place using [DataFusion](https://datafusion.apache.org/), an embedded columnar query engine. This means the first query is fast — there is no ingestion pipeline to wait for.
 
### The query
 
An engineer opens the **Cairn** frontend, a native desktop app built in Rust with [egui](https://crates.io/crates/egui). The frontend fetches the available schema and obstacle classes from the backend on startup and presents them as interactive filters. The engineer selects one or more obstacle classes — e.g. *car* and *person* — sets a minimum deceleration threshold, and clicks Replay.
 
The frontend sends that query to the backend as a single HTTP request. The backend translates it into a DataFusion SQL query that joins ego motion, obstacle detections, and LiDAR availability across the dataset, and applies a `HAVING COUNT(DISTINCT label_class)` condition to find clips containing **all** selected classes. It returns matching clip UUIDs in a few seconds — scanning potentially hundreds of gigabytes of Parquet with [predicate pushdown](https://datafusion.apache.org/user-guide/sql/special_functions/pushdown.html) rather than loading it all into memory.
 
### The replay
 
For each matching clip the backend fetches ego motion samples, [Draco-decoded](https://opensource.googleblog.com/2017/01/introducing-draco-compression-for-3d.html) LiDAR point clouds, and per-frame obstacle bounding boxes, then streams them all into a running [Rerun](https://rerun.io/) viewer. The result is a scrubable 3D timeline showing:
 
- The vehicle's path as a line strip
- Rotating LiDAR scans as point clouds
- Tracked obstacle bounding boxes that appear, move, and disappear in sync with detections
 
All sensor streams are locked to the same `ego_time` timeline so they are temporally aligned. The engineer can scrub through the clip, inspect individual frames, and immediately understand the driving context of the scenario they queried for.
 
---
 
## Why?
 
The primary objective of Cairn is **scenario mining**. Training [multimodal machine learning](https://en.wikipedia.org/wiki/Multimodal_learning) models for autonomous vehicles requires large amounts of diverse data. If the scenarios a model trains on are not sufficiently diverse, it leads to [overfitting](https://en.wikipedia.org/wiki/Overfitting) — high accuracy on training data but poor generalization to new scenarios.
 
Cairn addresses this by making it fast and interactive to find specific driving scenarios within a large dataset. For example: *find all clips where the vehicle was decelerating above 2.5 m/s² that contain both a car and a person*. This is done using a compiled, async Rust service with DataFusion as an embedded columnar query engine. This means:
 
- **Fast predicate pushdown** over large Parquet datasets — filtering is performed on the source files, not in memory
- **Concurrent queries** without the overhead of a Python runtime
- **Live streaming** of query results directly into a 3D visualization tool for immediate validation
 
---
 
## How?
 
### Prerequisites
 
- Rust 1.88+
- [cmake](https://cmake.org/) — required for Draco C++ point cloud bindings: `brew install cmake`
- A HuggingFace account with the [NVIDIA PhysicalAI-AV license accepted](https://huggingface.co/datasets/nvidia/PhysicalAI-Autonomous-Vehicles)
- [Rerun viewer](https://rerun.io/): `cargo install rerun-cli --locked`
 
### Setup
 
Clone the repo
```bash
git clone https://github.com/Vinnstah/cairn
cd cairn
```
 
Download a subset of the dataset from HuggingFace and place it under `./data/nvidia_physical_dataset`.
 
**Run the backend**
```bash
cargo run -p cairn
```
 
**Once the backend has registered its tables, run the frontend**
```bash
cargo run -p cairn-ui
```
 
> **Note:** Further improvements to the startup procedure are planned.
 
### Querying without the frontend
 
**Search for matching clip IDs or replay matching clips into Rerun**
```bash
curl "http://localhost:3000/clips/search?min_speed=15.0&min_decel=2.5"
 
curl "http://localhost:3000/clips/replay?min_speed=15.0&min_decel=2.5"
```
 
Each request queries the Parquet files via DataFusion SQL, finds matching clip UUIDs, then streams their ego motion trajectory and Draco-decoded LiDAR point clouds into the running Rerun viewer.
 
---

## Architecture
The app is split into 3 different crate: *backend*, *frontend* and *shared*.

- **`shared`** — types shared between frontend and backend (`ClipSearchParams`, `ColumnInfo`, `SchemaResponse`, `CairnError`)
- **`backend`** — ports-and-adapters ([hexagonal](https://alistair.cockburn.us/hexagonal-architecture)) architecture; core domain has no infrastructure dependencies
- **`frontend`** — native desktop GUI built with [eframe](https://crates.io/crates/eframe) and egui

### Dataset layout expected on disk

```
./data/nvidia_physical_dataset/
├── egomotion.chunk_0000/
│   └── <clip_uuid>.egomotion.parquet
│       # timestamp (relative µs), x, y, z, qx, qy, qz, qw
│       # vx, vy, vz, ax, ay, az, curvature — ~2500 rows/clip at ~100 Hz
├── camera_front_wide_120fov.chunk_0000/    # not yet fully supported
│   ├── <clip_uuid>.camera_front_wide_120fov.mp4
│   └── <clip_uuid>.camera_front_wide_120fov.timestamps.parquet
├── lidar.chunk_0000/
│   └── <clip_uuid>.lidar_top_360fov.parquet
│       # spin_index, spin_start_timestamp, draco_encoded_pointcloud
│       # ~200 spins/clip at 10 Hz
├── obstacle.offline.chunk_0000/
│   └── <clip_uuid>.obstacle.offline.parquet
│       # timestamp_us, track_id, label_class, center_x/y/z, size_x/y/z
│       # orientation quaternion, reference_frame
└── metadata/
    └── data_collection.parquet
        # clip_id, country, month, hour_of_day, platform_class
```
 
### Notable implementation details
 
**Clip ID injection** — The NVIDIA dataset stores clip identity only in filenames (`<uuid>.egomotion.parquet`), not as a column inside the Parquet files. Cairn works around this by registering each file individually as a sub-table and combining them into a `CREATE VIEW` with the UUID injected as a literal `clip_id` column, making cross-table joins possible without modifying the raw data.
 
**Draco decoding** — LiDAR point clouds are stored as Draco-compressed binary blobs in `BinaryView` Arrow columns. The `spatial_codec_draco` crate requires a color attribute that this dataset does not provide. Cairn uses `draco-rs` (Rust bindings to the C++ Draco library, requires `cmake`) which decodes geometry-only point clouds via `PointCloud::from_buffer` and `get_point_alloc::<f32, 3>`.
 
**Obstacle tracking in Rerun** — Each tracked obstacle is logged to its own entity path (`world/obstacles/<label_class>/<track_id>`). This uses Rerun's "latest at" semantics so each box persists until its next update. A `Clear::flat()` is logged just before the next detection for that track so boxes disappear when the object leaves the sensor's field of view rather than persisting indefinitely.
 
**Orphan rule workaround** — Implementing `TryFrom<RecordBatch> for Vec<PointCloud>` violates Rust's orphan rule since both types are foreign. Cairn uses a `PointClouds(Vec<PointCloud>)` newtype wrapper to implement the conversion cleanly.
 
**Arrow version isolation** — Rerun and DataFusion both re-export Arrow but pin different versions. All `RecordBatch` processing uses `datafusion::arrow` types exclusively. Rerun types (`Points3D`, `Transform3D`, `Boxes3D`) are constructed from plain Rust primitives at the `SceneLogger` adapter boundary.
