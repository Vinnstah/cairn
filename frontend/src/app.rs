use crate::client::{fetch_schema, trigger_replay};
use egui::{Color32, RichText, ScrollArea, Ui};
use shared::{ClipSearchParams, ColumnInfo};
use std::collections::HashSet;

const NUMERIC_TYPES: &[&str] = &["Float32", "Float64", "Int32", "Int64"];

fn is_numeric(data_type: &str) -> bool {
    NUMERIC_TYPES.iter().any(|t| data_type.contains(t))
}

#[derive(Default, Clone)]
struct ColumnFilter {
    enabled: bool,
    min: String,
    max: String,
}

pub struct CairnApp {
    columns: Vec<ColumnInfo>,
    label_classes: Vec<String>,
    selected_label_classes: std::collections::HashSet<String>,
    filters: std::collections::HashMap<String, ColumnFilter>,
    schema_error: Option<String>,
    min_speed: String,
    min_decel: String,
    replaying: bool,
    replay_status: Option<(String, bool)>,
    backend_url: String,
}

impl CairnApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let (columns, label_classes, schema_error) = match fetch_schema() {
            Ok(schema) => (schema.column_info, schema.label_classes, None),
            Err(e) => (
                vec![],
                vec![],
                Some(format!("Could not reach backend: {}", e)),
            ),
        };

        let filters = columns
            .iter()
            .map(|c| (c.name.clone(), ColumnFilter::default()))
            .collect();

        Self {
            columns,
            label_classes,
            selected_label_classes: HashSet::new(),
            filters,
            schema_error,
            min_speed: String::new(),
            min_decel: String::new(),
            replaying: false,
            replay_status: None,
            backend_url: "http://localhost:3000".into(),
        }
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("🚗  Cairn").strong());
            ui.separator();
            ui.label("AV Scenario Explorer");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⟳  Refresh schema").clicked() {
                    match fetch_schema() {
                        Ok(schema) => {
                            self.filters = schema
                                .column_info
                                .iter()
                                .map(|c| (c.name.clone(), ColumnFilter::default()))
                                .collect();
                            self.columns = schema.column_info;
                            self.schema_error = None;
                        }
                        Err(e) => {
                            self.schema_error = Some(format!("Refresh failed: {}", e));
                        }
                    }
                }
                ui.add(egui::TextEdit::singleline(&mut self.backend_url).desired_width(200.0));
                ui.label(RichText::new("Backend:").weak());
            });
        });
        ui.add_space(4.0);
    }

    fn right_panel(&mut self, ui: &mut Ui) {
        ui.add_space(8.0);

        if let Some(err) = &self.schema_error.clone() {
            ui.colored_label(Color32::RED, format!("⚠  {}", err));
            ui.add_space(4.0);
            ui.separator();
        }

        ui.label(RichText::new("Quick filters").strong().size(14.0));
        ui.add_space(4.0);

        egui::Grid::new("quick_filters")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("min speed (m/s)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.min_speed)
                        .desired_width(80.0)
                        .hint_text("e.g. 15.0"),
                );
                ui.end_row();

                ui.label("min decel (m/s²)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.min_decel)
                        .desired_width(80.0)
                        .hint_text("e.g. 2.5"),
                );
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();

        ui.label(RichText::new("ego_motion columns").strong().size(14.0));
        ui.add_space(2.0);
        ui.label(
            RichText::new("Enable columns to add per-column range filters")
                .weak()
                .size(11.0),
        );
        ui.add_space(6.0);

        let available_height = ui.available_height() - 80.0;
        ScrollArea::vertical()
            .max_height(available_height)
            .id_salt("col_scroll")
            .show(ui, |ui| {
                let columns = self.columns.clone();
                for col in &columns {
                    self.column_row(ui, col);
                }
            });

        ui.separator();
        ui.label(RichText::new("Obstacle classes").strong().size(14.0));
        ui.add_space(4.0);

        ScrollArea::vertical()
            .max_height(150.0)
            .id_salt("label_scroll")
            .show(ui, |ui| {
                let classes = self.label_classes.clone();
                for class in &classes {
                    let mut selected = self.selected_label_classes.contains(class);
                    if ui.checkbox(&mut selected, class).changed() {
                        if selected {
                            self.selected_label_classes.insert(class.clone());
                        } else {
                            self.selected_label_classes.remove(class);
                        }
                    }
                }
            });

        ui.separator();
        self.replay_button(ui);
        ui.add_space(8.0);
    }

    fn column_row(&mut self, ui: &mut Ui, col: &ColumnInfo) {
        let filter = self.filters.entry(col.name.clone()).or_default();
        let numeric = is_numeric(&col.data_type);

        ui.horizontal(|ui| {
            ui.add_enabled_ui(numeric, |ui| {
                ui.checkbox(&mut filter.enabled, "");
            });

            ui.vertical(|ui| {
                ui.label(RichText::new(&col.name).monospace());
                ui.label(
                    RichText::new(&col.data_type)
                        .weak()
                        .size(10.0)
                        .color(if numeric {
                            Color32::from_rgb(100, 180, 100)
                        } else {
                            Color32::GRAY
                        }),
                );
            });

            if filter.enabled && numeric {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut filter.max)
                            .desired_width(55.0)
                            .hint_text("max"),
                    );
                    ui.label("–");
                    ui.add(
                        egui::TextEdit::singleline(&mut filter.min)
                            .desired_width(55.0)
                            .hint_text("min"),
                    );
                });
            }
        });

        ui.add_space(2.0);
    }

    fn replay_button(&mut self, ui: &mut Ui) {
        ui.add_space(8.0);

        let label = if self.replaying {
            RichText::new("⏳  Replaying...").size(15.0)
        } else {
            RichText::new("▶  Replay in Rerun").size(15.0)
        };

        let btn = egui::Button::new(label).min_size(egui::vec2(ui.available_width(), 36.0));

        if ui.add_enabled(!self.replaying, btn).clicked() {
            self.replaying = true;
            self.replay_status = None;

            let params = ClipSearchParams {
                min_speed: self.min_speed.parse::<f64>().ok(),
                min_decel: self.min_decel.parse::<f64>().ok(),
                label_classes: self.selected_label_classes.iter().cloned().collect(),
            };

            match trigger_replay(&params) {
                Ok(_) => {
                    self.replay_status = Some(("✓  Replay triggered — check Rerun".into(), true))
                }
                Err(e) => self.replay_status = Some((format!("✗  {}", e), false)),
            }
            self.replaying = false;
        }

        if let Some((msg, ok)) = &self.replay_status {
            ui.add_space(6.0);
            ui.colored_label(if *ok { Color32::GREEN } else { Color32::RED }, msg);
        }
    }

    fn centre_panel(&self, ui: &mut Ui) {
        let active: Vec<_> = self.filters.iter().filter(|(_, f)| f.enabled).collect();

        if active.is_empty()
            && self.min_speed.is_empty()
            && self.min_decel.is_empty()
            && self.selected_label_classes.is_empty()
        {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.label(RichText::new("🔍").size(48.0));
                    ui.add_space(12.0);
                    ui.label(RichText::new("No filters selected").size(20.0).weak());
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(
                            "Set a quick filter on the right, or enable columns\n\
                             to add per-column range filters.\n\n\
                             Then click ▶ Replay — results will appear in Rerun.",
                        )
                        .weak()
                        .size(14.0),
                    );
                    ui.add_space(40.0);
                    ui.separator();
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("Make sure the Rerun viewer is running:")
                            .weak()
                            .size(13.0),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("rerun")
                            .monospace()
                            .size(13.0)
                            .color(Color32::from_rgb(180, 180, 100)),
                    );
                });
            });
        } else {
            ui.add_space(16.0);
            ui.label(RichText::new("Active query").strong().size(16.0));
            ui.add_space(8.0);

            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                ui.add_space(8.0);

                if !self.min_speed.is_empty() {
                    ui.label(
                        RichText::new(format!("AVG speed  >  {} m/s", self.min_speed)).monospace(),
                    );
                }
                if !self.min_decel.is_empty() {
                    ui.label(
                        RichText::new(format!("AVG decel  >  {} m/s²", self.min_decel)).monospace(),
                    );
                }

                for (name, filter) in &active {
                    let parts = match (filter.min.is_empty(), filter.max.is_empty()) {
                        (false, false) => format!("{}  in  [{}, {}]", name, filter.min, filter.max),
                        (false, true) => format!("{}  ≥  {}", name, filter.min),
                        (true, false) => format!("{}  ≤  {}", name, filter.max),
                        (true, true) => format!("{} (selected, no range)", name),
                    };
                    ui.label(RichText::new(parts).monospace());
                }
                ui.add_space(8.0);
                if !self.selected_label_classes.is_empty() {
                    let classes = self
                        .selected_label_classes
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    ui.label(RichText::new(format!("obstacles  ∈  [{}]", classes)).monospace());
                }
            });

            ui.add_space(24.0);
            ui.label(
                RichText::new(
                    "Click ▶ Replay to send this query to the backend.\n\
                     Matching clips will stream into Rerun.",
                )
                .weak(),
            );
        }
    }
}

impl eframe::App for CairnApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top_bar").show_inside(ui, |ui| {
            self.top_bar(ui);
        });

        egui::Panel::right("controls")
            .resizable(true)
            .min_size(280.0)
            .max_size(400.0)
            .show_inside(ui, |ui| {
                self.right_panel(ui);
            });

        egui::Panel::top("top").show_inside(ui, |ui| {
            self.centre_panel(ui);
        });
    }
}
