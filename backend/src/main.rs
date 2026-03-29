use crate::startup::start;

pub mod adapters;
pub mod core;
pub mod error;
mod startup;

#[tokio::main]
async fn main() {
    start().await;
}
