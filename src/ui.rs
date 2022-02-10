use bevy::prelude::*;
use bevy_egui::egui;

use crate::{events::GameEvent, plant::*};
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(ui_system);
    }
}

fn ui_system(
    mut ctx: ResMut<bevy_egui::EguiContext>,
    mut plants: Query<(Entity, &mut OptionsComponent, &PlantInfoComponent), With<PlantComponent>>,
    mut events: EventWriter<GameEvent>,
) {
    // egui::TopBottomPanel::bottom("info").show(ctx.ctx_mut(), |ui| {

    // });

    let mut i = 0;
    plants.for_each_mut(|(entity, mut values, PlantInfoComponent { line_count })| {
        i += 1;
        egui::Window::new(format!("Plant {i} settings"))
            .collapsible(false)
            .default_pos(egui::Pos2::new(32., 32.))
            .resizable(false)
            .show(ctx.ctx_mut(), |ui| {
                ui.label(format!("Total polylines: {line_count}"));

                ui.separator();

                ui.label("Draw method");
                let render_checkbox = ui.checkbox(
                    &mut values.expensive_rendering,
                    "Use expensive rendering mode (may cause issues)",
                );
                ui.small("Disabling this enables more iterations");

                ui.separator();

                ui.style_mut().spacing.slider_width = 300.;
                // Show render settings
                ui.label("Iterations");
                let iter_range = {
                    if values.expensive_rendering {
                        1..=7
                    } else {
                        1..=10
                    }
                };
                let iterations = ui.add(egui::Slider::new(&mut values.iterations, iter_range));

                ui.separator();

                ui.label("Rotation angle");
                let rot_angle = ui.add(
                    egui::Slider::new(&mut values.rotation_amount, 0f32..=180f32)
                        .max_decimals(2)
                        .smart_aim(false),
                );

                ui.separator();

                ui.label("Segment length");
                let segment_length = ui.add(egui::Slider::new(
                    &mut values.segment_length,
                    0.01f32..=1.0f32,
                ));

                ui.separator();

                ui.label("Axiom");
                let axiom = ui.add(
                    egui::TextEdit::multiline(&mut values.axiom)
                        .code_editor()
                        .desired_rows(1)
                        .desired_width(f32::INFINITY),
                );

                ui.separator();

                // Show rules text
                ui.label("Rules");
                let mut rule_changed = false;
                let mut to_remove = Vec::with_capacity(values.rules.len());
                for (i, rule) in values.rules.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        let text = ui.add(
                            egui::TextEdit::multiline(rule)
                                .code_editor()
                                .desired_rows(1)
                                .desired_width(f32::INFINITY),
                        );
                        let remove_button = ui.button("Remove rule");
                        if remove_button.clicked() {
                            to_remove.push(i);
                        }
                        if text.changed() || remove_button.clicked() {
                            rule_changed = true;
                        }
                    });
                    ui.separator();
                }
                for i in to_remove {
                    values.rules.remove(i);
                }

                let add_rule = ui.button("Add rule");

                if add_rule.clicked() {
                    values.rules.push(Default::default());
                };

                if rot_angle.changed()
                    || segment_length.changed()
                    || iterations.changed()
                    || add_rule.clicked()
                    || axiom.changed()
                    || render_checkbox.changed()
                    || rule_changed
                {
                    events.send(GameEvent::TriggerUpdate(entity))
                }
            });
    });
}

#[derive(Component)]
pub struct OptionsComponent {
    pub rotation_amount: f32,
    pub segment_length: f32,
    pub rules: Vec<String>,
    pub axiom: String,
    pub iterations: usize,
    pub expensive_rendering: bool,
}

impl Default for OptionsComponent {
    fn default() -> Self {
        Self {
            rotation_amount: 20f32,
            segment_length: 0.5f32,
            rules: vec![String::from("X=[+F][-F]FX"), String::from("F=FX")],
            axiom: String::from("X"),
            iterations: 5,
            expensive_rendering: false,
        }
    }
}
