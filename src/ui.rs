use bevy::prelude::*;
use bevy_egui::egui::{self, color::Hsva};

use crate::{
    events::GameEvent,
    plant::{self, *},
};
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(ui_system);
    }
}

fn ui_system(
    mut ctx: ResMut<bevy_egui::EguiContext>,
    mut plants: Query<(
        Entity,
        &mut OptionsComponent,
        &PlantStatsComponent,
        &PlantComponent,
        &mut Transform,
    )>,
    mut events: EventWriter<GameEvent>,
    mut selected: ResMut<SelectedPlantsResource>,
    mut offset: Local<Vec3>,
) {
    egui::TopBottomPanel::bottom("info").show(ctx.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Add plant").clicked() {
                *offset = *offset + Vec3::X * 2f32;
                events.send(GameEvent::SpawnNew(Transform::from_translation(*offset)));
            }

            ui.separator();

            plants.for_each(|(e, ..)| {
                if ui.button(format!("Plant {}", e.id() - 1)).clicked() {
                    selected.0.insert(e, ());
                }
            });
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Rotate: Middle click and drag");
                ui.separator();
                ui.label("Pan: Right click and drag");
                ui.separator();
                ui.label("Zoom: Scroll in and out");
            });
        });
    });

    let mut i = 0;
    plants.for_each_mut(
        |(entity, mut values, PlantStatsComponent { vert_count }, plant, mut transform)| {
            i += 1;
            let is_selected = selected.0.iter().any(|(x, _)| *x == entity);
            let mut window_is_open = is_selected;
            egui::Window::new(format!("Plant {i} settings"))
                .collapsible(false)
                .resizable(false)
                .open(&mut window_is_open)
                .show(ctx.ctx_mut(), |ui| {
                    ui.label(format!("Total verticies: {vert_count}"));

                    ui.collapsing("Settings", |ui| {
                        ui.label("Line color");
                        let color = ui.color_edit_button_hsva(&mut values.line_color);

                        ui.separator();

                        ui.label("Line width");
                        let width = ui.add(
                            egui::Slider::new(&mut values.line_width, 0.1f32..=500f32)
                                .smart_aim(false)
                                .max_decimals(2),
                        );

                        ui.separator();

                        ui.label("Transform");
                        let mut translation: Vec3 = transform.translation.clone();
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut translation.x)
                                    .prefix("x: ")
                                    .min_decimals(2),
                            );
                            ui.add(
                                egui::DragValue::new(&mut translation.y)
                                    .prefix("y: ")
                                    .min_decimals(2),
                            );
                            ui.add(
                                egui::DragValue::new(&mut translation.z)
                                    .prefix("z: ")
                                    .min_decimals(2),
                            );
                        });
                        transform.translation = translation;

                        ui.separator();

                        ui.style_mut().spacing.slider_width = 300.;
                        // Show render settings
                        ui.label("Iterations");

                        let iterations = ui.add(egui::Slider::new(&mut values.iterations, 1..=10));

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

                        if rot_angle.changed()
                            || segment_length.changed()
                            || iterations.changed()
                            || color.changed()
                            || width.changed()
                        {
                            events.send(GameEvent::TriggerUpdate(entity))
                        }
                    });

                    ui.separator();

                    egui::CollapsingHeader::new("Rules")
                        .default_open(true)
                        .show(ui, |ui| {
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

                            if add_rule.clicked() || axiom.changed() || rule_changed {
                                events.send(GameEvent::TriggerUpdate(entity))
                            }
                        });

                    ui.separator();

                    ui.collapsing("Symbols", |ui| {
                        let mut chars: Vec<&char> =
                            plant.action_map.iter().map(|(c, _)| c).collect();
                        chars.sort();
                        egui::Grid::new(format!("grid {:?}", entity)).show(ui, |ui| {
                            for token in chars.iter() {
                                let mut token_text = token.to_string();
                                let token_edit = ui.text_edit_singleline(&mut token_text);

                                if token_edit.changed() && !token_text.is_empty() {
                                    let next = token_text.chars().next().unwrap_or(**token);
                                    events.send(GameEvent::ChangeToken {
                                        entity,
                                        prev: **token,
                                        next,
                                    });
                                }

                                let mut current_action =
                                    plant.action_map.get(&token).unwrap().clone();
                                egui::ComboBox::from_id_source(format!("{:?}{}", entity, token))
                                    .selected_text(current_action.to_string())
                                    .show_ui(ui, |ui| {
                                        let mut select = |current: &mut Action, action: Action| {
                                            if ui
                                                .selectable_value(
                                                    current,
                                                    action,
                                                    action.to_string(),
                                                )
                                                .clicked()
                                            {
                                                events.send(GameEvent::ChangeAction {
                                                    entity,
                                                    token: **token,
                                                    action: action,
                                                })
                                            }
                                        };

                                        select(&mut current_action, Action::Nothing);
                                        select(&mut current_action, Action::Forwards);
                                        select(
                                            &mut current_action,
                                            Action::Rotate(plant::Direction::XPos),
                                        );
                                        select(
                                            &mut current_action,
                                            Action::Rotate(plant::Direction::XNeg),
                                        );
                                        select(
                                            &mut current_action,
                                            Action::Rotate(plant::Direction::YPos),
                                        );
                                        select(
                                            &mut current_action,
                                            Action::Rotate(plant::Direction::YNeg),
                                        );
                                        select(
                                            &mut current_action,
                                            Action::Rotate(plant::Direction::ZPos),
                                        );
                                        select(
                                            &mut current_action,
                                            Action::Rotate(plant::Direction::ZNeg),
                                        );
                                        select(&mut current_action, Action::Push);
                                        select(&mut current_action, Action::Pop);
                                    })
                                    .response;

                                if ui.button("Remove").clicked() {
                                    events.send(GameEvent::RemoveToken {
                                        entity,
                                        token: **token,
                                    });
                                }
                                ui.end_row();
                            }
                        });
                        if ui.button("Add symbol").clicked() {
                            events.send(GameEvent::AddToken {
                                entity,
                                token: '~',
                                action: Action::Nothing,
                            });
                        }
                    });
                });

            // if the window has closed this frame
            if window_is_open == false && is_selected {
                selected.0.remove(&entity);
            }
        },
    );
}

#[derive(Component)]
pub struct OptionsComponent {
    pub rotation_amount: f32,
    pub segment_length: f32,
    pub rules: Vec<String>,
    pub axiom: String,
    pub iterations: usize,
    pub line_width: f32,
    pub line_color: Hsva,
}

impl Default for OptionsComponent {
    fn default() -> Self {
        Self {
            rotation_amount: 30f32,
            segment_length: 0.25f32,
            rules: vec![String::from("X=[+F][^F][-F][vF]FX"), String::from("F=FX")],
            axiom: String::from("X"),
            iterations: 6,
            line_width: 10f32,
            line_color: Hsva::from_rgb([0f32, 1f32, 0.1f32]),
        }
    }
}
