mod app;
mod client;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cairn — AV Scenario Explorer")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Cairn",
        native_options,
        Box::new(|cc| Ok(Box::new(app::CairnApp::new(cc)))),
    )
}
