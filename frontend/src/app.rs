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
    selected_label_classes: HashSet<String>,
    filters: std::collections::HashMap<String, ColumnFilter>,
    schema_error: Option<String>,
    min_speed: String,
    min_decel: String,
    replaying: bool,
    replay_status: Option<(String, bool)>,
    backend_url: String,
    columns_expanded: bool,
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
            columns_expanded: false,
        }
    }

    // ── Top bar ───────────────────────────────────────────────────────────────

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("🚗  Cairn").strong());
            ui.separator();
            ui.label(RichText::new("AV Scenario Explorer").weak());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⟳  Refresh schema").clicked() {
                    match fetch_schema() {
                        Ok(schema) => {
                            self.filters = schema
                                .column_info
                                .iter()
                                .map(|c| (c.name.clone(), ColumnFilter::default()))
                                .collect();
                            self.label_classes = schema.label_classes;
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

    // ── Centre panel — three zones stacked vertically ─────────────────────────

    fn centre_panel(&mut self, ui: &mut Ui) {
        if let Some(err) = &self.schema_error.clone() {
            ui.colored_label(Color32::RED, format!("⚠  {}", err));
            ui.separator();
        }

        // ── 1. Replay bar (fixed bottom) ──────────────────────────────────
        egui::Panel::bottom("replay_bar")
            .resizable(false)
            .min_size(52.0)
            .show_inside(ui, |ui| {
                self.replay_bar(ui);
            });

        // ── 2. Ego motion columns (collapsible bottom) ────────────────────
        egui::Panel::bottom("columns_panel")
            .resizable(true)
            .min_size(if self.columns_expanded { 160.0 } else { 36.0 })
            .max_size(300.0)
            .show_inside(ui, |ui| {
                self.columns_section(ui);
            });

        // ── 3. Quick filters (fixed top) ──────────────────────────────────
        egui::Panel::top("quick_filters_panel")
            .resizable(false)
            .show_inside(ui, |ui| {
                self.quick_filters_section(ui);
            });

        // ── 4. Label class buttons (remaining centre space) ───────────────
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.label_classes_section(ui);
        });
    }

    // ── Quick filters ─────────────────────────────────────────────────────────

    fn quick_filters_section(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.label(RichText::new("Filters").strong().size(13.0));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Min speed").size(12.0).weak());
            ui.add(
                egui::TextEdit::singleline(&mut self.min_speed)
                    .desired_width(65.0)
                    .hint_text("m/s"),
            );

            ui.add_space(20.0);

            ui.label(RichText::new("Min decel").size(12.0).weak());
            ui.add(
                egui::TextEdit::singleline(&mut self.min_decel)
                    .desired_width(65.0)
                    .hint_text("m/s²"),
            );

            // Active chips
            ui.add_space(16.0);
            if !self.min_speed.is_empty() {
                active_chip(ui, &format!("speed > {}", self.min_speed));
            }
            if !self.min_decel.is_empty() {
                active_chip(ui, &format!("decel > {}", self.min_decel));
            }
            for class in &self
                .selected_label_classes
                .iter()
                .cloned()
                .collect::<Vec<_>>()
            {
                active_chip(ui, class);
            }
        });

        ui.add_space(10.0);
        ui.separator();
    }

    // ── Label class pill buttons ──────────────────────────────────────────────

    fn label_classes_section(&mut self, ui: &mut Ui) {
        ui.add_space(16.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Obstacle classes").strong().size(14.0));
            ui.label(
                RichText::new("— clips must contain ALL selected")
                    .weak()
                    .size(11.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.selected_label_classes.is_empty() {
                    if ui.small_button("✕  clear all").clicked() {
                        self.selected_label_classes.clear();
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("{} selected", self.selected_label_classes.len()))
                            .size(11.0)
                            .color(Color32::from_rgb(100, 180, 255)),
                    );
                }
            });
        });

        ui.add_space(12.0);

        if self.label_classes.is_empty() {
            ui.label(RichText::new("No obstacle classes found — is the backend running?").weak());
            return;
        }

        ScrollArea::vertical()
            .id_salt("label_scroll")
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    let classes = self.label_classes.clone();
                    for class in &classes {
                        let selected = self.selected_label_classes.contains(class);
                        if class_pill(ui, class, selected).clicked() {
                            if selected {
                                self.selected_label_classes.remove(class);
                            } else {
                                self.selected_label_classes.insert(class.clone());
                            }
                        }
                    }
                });
            });
    }

    // ── Ego motion columns (collapsible) ──────────────────────────────────────

    fn columns_section(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let arrow = if self.columns_expanded { "▾" } else { "▸" };
            if ui
                .button(
                    RichText::new(format!("{}  ego_motion columns", arrow))
                        .strong()
                        .size(12.0),
                )
                .clicked()
            {
                self.columns_expanded = !self.columns_expanded;
            }

            let active_count = self.filters.values().filter(|f| f.enabled).count();
            if active_count > 0 {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{} active", active_count))
                        .size(11.0)
                        .color(Color32::from_rgb(100, 180, 255)),
                );
            }
        });

        if !self.columns_expanded {
            return;
        }

        ui.add_space(4.0);
        ui.label(
            RichText::new("Enable numeric columns to add range filters")
                .weak()
                .size(11.0),
        );
        ui.add_space(4.0);

        ScrollArea::vertical().id_salt("col_scroll").show(ui, |ui| {
            let columns = self.columns.clone();
            for col in &columns {
                self.column_row(ui, col);
            }
        });
    }

    fn column_row(&mut self, ui: &mut Ui, col: &ColumnInfo) {
        let filter = self.filters.entry(col.name.clone()).or_default();
        let numeric = is_numeric(&col.data_type);

        ui.horizontal(|ui| {
            ui.add_enabled_ui(numeric, |ui| {
                ui.checkbox(&mut filter.enabled, "");
            });

            ui.label(RichText::new(&col.name).monospace().size(11.0));

            ui.label(
                RichText::new(&col.data_type)
                    .weak()
                    .size(10.0)
                    .color(if numeric {
                        Color32::from_rgb(100, 180, 100)
                    } else {
                        Color32::from_gray(100)
                    }),
            );

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

        ui.add_space(1.0);
    }

    // ── Replay bar ────────────────────────────────────────────────────────────

    fn replay_bar(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let label = if self.replaying {
                RichText::new("⏳  Replaying...").size(15.0)
            } else {
                RichText::new("▶  Replay in Rerun").size(15.0)
            };

            let btn = egui::Button::new(label).min_size(egui::vec2(200.0, 36.0));

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
                        self.replay_status =
                            Some(("✓  Replay triggered — check Rerun".into(), true))
                    }
                    Err(e) => self.replay_status = Some((format!("✗  {}", e), false)),
                }
                self.replaying = false;
            }

            if let Some((msg, ok)) = &self.replay_status {
                ui.add_space(12.0);
                ui.colored_label(
                    if *ok {
                        Color32::from_rgb(100, 200, 100)
                    } else {
                        Color32::RED
                    },
                    msg,
                );
            }
        });
    }
}

// ── Standalone widget helpers ─────────────────────────────────────────────────

/// Pill-shaped toggle button for label classes.
/// Returns the egui Response so the caller can check .clicked().
fn class_pill(ui: &mut Ui, label: &str, selected: bool) -> egui::Response {
    let (bg, fg, stroke) = if selected {
        (
            Color32::from_rgb(30, 70, 140),
            Color32::WHITE,
            egui::Stroke::new(1.5, Color32::from_rgb(80, 140, 255)),
        )
    } else {
        (
            Color32::from_gray(42),
            Color32::from_gray(200),
            egui::Stroke::new(1.0, Color32::from_gray(70)),
        )
    };

    ui.add(
        egui::Button::new(RichText::new(label).size(12.0).color(fg))
            .fill(bg)
            .stroke(stroke)
            .corner_radius(16.0)
            .min_size(egui::vec2(0.0, 28.0)),
    )
}

/// Small non-interactive chip showing an active filter value.
fn active_chip(ui: &mut Ui, label: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(20, 55, 20))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 130, 60)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(label)
                    .size(11.0)
                    .color(Color32::from_rgb(100, 210, 100)),
            );
        });
}

impl eframe::App for CairnApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top_bar").show_inside(ui, |ui| {
            self.top_bar(ui);
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.centre_panel(ui);
        });
    }
}
