use crate::compiler::{compile, interpret_ir, Inst};
use crate::gui::CodeEditor;
use eframe::egui;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct LndwApp {
    code_editor: CodeEditor,
    unoptimized_asm: Vec<(Option<f32>, Inst)>,
    optimized_asm: Vec<(Option<f32>, Inst)>,
    input_variables: HashMap<String, String>,
    result: Option<String>,
    error_message: Option<String>,
}

impl LndwApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        cc.egui_ctx.set_zoom_factor(1.5);
        Self::default()
    }
}

impl eframe::App for LndwApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::right("original_panel")
            .resizable(true)
            .default_width(300.0)
            .min_width(250.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Original Assembly");
                });
                ui.separator();

                if let Some(error) = &self.error_message {
                    ui.colored_label(egui::Color32::RED, "Error:");
                    ui.colored_label(egui::Color32::RED, error);
                } else {
                    egui::ScrollArea::vertical()
                        .id_source("original_scroll")
                        .max_height(ui.available_height() - 40.0)
                        .show(ui, |ui| {
                            egui::Grid::new("unoptimized_output")
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .min_col_width(30.0)
                                .show(ui, |ui| {
                                    let mut should_update = true;
                                    for (progress, inst) in &mut self.unoptimized_asm {
                                        let progress_increment = match inst {
                                            Inst::Add(_, _) => 0.05,
                                            Inst::Sub(_, _) => 0.05,
                                            Inst::Mul(_, _) => 0.025,
                                            Inst::Div(_, _) => 0.01,
                                            Inst::Store(_, _) => 0.1,
                                            Inst::Transfer(_, _) => 0.1,
                                            Inst::Result(_) => 0.1,
                                        };
                                        if should_update && progress.is_none() {
                                            *progress = Some(progress_increment);
                                            should_update = false;
                                        }
                                        if should_update
                                            && progress.is_some()
                                            && progress.unwrap() < 1.0
                                        {
                                            *progress =
                                                Some(progress.unwrap() + progress_increment);
                                            should_update = false;
                                        }
                                        let bar = egui::ProgressBar::new(progress.unwrap_or(0.0))
                                            .animate(true)
                                            .desired_width(30.0)
                                            .desired_height(7.5);
                                        ui.add_visible(progress.is_some(), bar);
                                        ui.label(format!("{inst}"));
                                        ui.end_row();
                                    }
                                });
                        });
                }
            });
        egui::SidePanel::right("optimized_panel")
            .resizable(true)
            .default_width(300.0)
            .min_width(250.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Optimized Assembly");
                });
                ui.separator();

                let has_optimizations = self.code_editor.do_constant_folding
                    || self.code_editor.do_dead_code_elimination
                    || self.code_editor.do_common_factor_extraction;

                if !has_optimizations {
                    ui.centered_and_justified(|ui| {
                        ui.label("No optimizations enabled");
                    });
                } else if let Some(error) = &self.error_message {
                    ui.colored_label(egui::Color32::RED, "Error:");
                    ui.colored_label(egui::Color32::RED, error);
                } else {
                    egui::ScrollArea::vertical()
                        .id_source("optimized_scroll")
                        .max_height(ui.available_height() - 40.0)
                        .show(ui, |ui| {
                            egui::Grid::new("optimized_output")
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .min_col_width(30.0)
                                .show(ui, |ui| {
                                    let mut should_update = true;
                                    for (progress, inst) in &mut self.optimized_asm {
                                        let progress_increment = match inst {
                                            Inst::Add(_, _) => 0.05,
                                            Inst::Sub(_, _) => 0.05,
                                            Inst::Mul(_, _) => 0.025,
                                            Inst::Div(_, _) => 0.01,
                                            Inst::Store(_, _) => 0.1,
                                            Inst::Transfer(_, _) => 0.1,
                                            Inst::Result(_) => 0.1,
                                        };
                                        if should_update && progress.is_none() {
                                            *progress = Some(progress_increment);
                                            should_update = false;
                                        }
                                        if should_update
                                            && progress.is_some()
                                            && progress.unwrap() < 1.0
                                        {
                                            *progress =
                                                Some(progress.unwrap() + progress_increment);
                                            should_update = false;
                                        }
                                        let bar = egui::ProgressBar::new(progress.unwrap_or(0.0))
                                            .animate(true)
                                            .desired_width(30.0)
                                            .desired_height(7.5);
                                        ui.add_visible(progress.is_some(), bar);
                                        ui.label(format!("{inst}"));
                                        ui.end_row();
                                    }
                                });
                        });
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello! Write some expression here");

            self.code_editor.ui(ui);

            if ui.button("Compile!").clicked() {
                self.result = None;
                self.error_message = None;

                match compile(
                    &self.code_editor.code,
                    self.code_editor.do_constant_folding,
                    self.code_editor.do_dead_code_elimination,
                    self.code_editor.do_common_factor_extraction,
                ) {
                    Ok(((unopt_asm, unopt_vars), (opt_asm, opt_vars))) => {
                        self.unoptimized_asm =
                            unopt_asm.iter().map(|i| (None, i.clone())).collect();
                        self.optimized_asm = opt_asm.iter().map(|i| (None, i.clone())).collect();
                        self.input_variables = unopt_vars
                            .iter()
                            .map(|s| (s.clone(), String::new()))
                            .collect();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("{e}"));
                        self.unoptimized_asm.clear();
                        self.optimized_asm.clear();
                        self.input_variables.clear();
                    }
                }
            }

            if !self.input_variables.is_empty() {
                ui.heading("Input variables:");
            }

            egui::Grid::new("vars").show(ui, |ui| {
                for (var, val) in self.input_variables.iter_mut() {
                    ui.label(var);
                    ui.text_edit_singleline(val);
                    ui.end_row();
                }
            });

            if ui.button("Run!").clicked() && self.error_message.is_none() {
                let instructions_to_run = if self.code_editor.do_constant_folding
                    || self.code_editor.do_dead_code_elimination
                    || self.code_editor.do_common_factor_extraction
                {
                    self.optimized_asm.iter().map(|i| i.1.clone()).collect()
                } else {
                    self.unoptimized_asm.iter().map(|i| i.1.clone()).collect()
                };

                let result = interpret_ir(instructions_to_run, &self.input_variables);
                match result {
                    Ok(r) => {
                        self.result = Some(r.to_string());
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.result = None;
                        self.error_message = Some(format!("{e}"));
                    }
                }
            }

            if let Some(result) = &self.result {
                ui.label(format!("Result: {result}"));
            }
        });
    }
}
