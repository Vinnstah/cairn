#[derive(Debug)]
pub struct DataError {
    pub error_msg: String,
}

impl DataError {
    pub fn new(error_msg: String) -> Self {
        Self { error_msg }
    }
}

#[derive(serde::Deserialize)]
pub struct ClipSearchParams {
    pub min_decel: Option<f64>,
    pub min_speed: Option<f64>,
}

#[derive(Clone)]
pub struct PointCloud {
    pub points: Vec<[f32; 3]>,
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
        PointCloud { points: value }
    }
}

#[derive(Clone)]
pub struct EgoMotion {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // qx, qy, qz, qw
}
