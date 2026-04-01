#[derive(Clone, Debug)]
pub struct PointCloud {
    pub points: Vec<[f32; 3]>,
    pub spin_start_timestamp: i64, // relative microseconds, same scale as ego/obstacles
}

impl IntoIterator for PointCloud {
    type Item = [f32; 3];

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.points.into_iter()
    }
}

impl From<Vec<[f32; 3]>> for PointCloud {
    fn from(value: Vec<[f32; 3]>) -> Self {
        PointCloud {
            points: value,
            spin_start_timestamp: 0,
        }
    }
}

#[derive(Clone)]
pub struct EgoMotion {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // qx, qy, qz, qw
}

#[derive(Clone)]
pub struct BoundingBox {
    pub track_id: String,
    pub label_class: String,
    pub timestamp_us: i64,
    pub center: [f32; 3],
    pub size: [f32; 3],
    pub rotation: [f32; 4], // qx, qy, qz, qw
}
