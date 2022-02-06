use bevy::prelude::*;
use bevy_egui::egui::{self, Align2};

use crate::plant::*;
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(ui_system);
    }
}

fn ui_system(
    mut cmd: Commands,
    mut ctx: ResMut<bevy_egui::EguiContext>,
    mut plants: Query<(
        Entity,
        &PlantComponent,
        &mut RulesComponent,
        &PlantBuilderComponent,
        &mut PlantRendererComponent,
    )>,
) {
    let mut i = 0;
    let mut dirty = false;
    plants.for_each_mut(|(entity, plant, mut rules, builder, mut render)| {
        i += 1;
        let mut render_dirty = false;
        let mut refresh = false;
        egui::Window::new(format!("Plant {i} settings"))
            .collapsible(false)
            .anchor(Align2::LEFT_TOP, egui::Vec2::ZERO)
            .show(ctx.ctx_mut(), |ui| {
                // Show render settings
                render_dirty = {
                    ui.add(
                        egui::Slider::new(&mut render.options.rotation_angle, 0f32..=180f32)
                            .max_decimals(2)
                            .smart_aim(false),
                    )
                    .changed()
                        || ui
                            .add(egui::Slider::new(
                                &mut render.options.segment_length,
                                0f32..=1f32,
                            ))
                            .changed()
                };

                // Show rules text
                for rule in rules.iter_mut() {
                    if ui
                        .add(
                            egui::TextEdit::multiline(rule)
                                .code_editor()
                                .desired_rows(2)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        dirty = true;
                    };
                }
            });
        if dirty {
            rules.dirty();
        }
        if render_dirty {
            render.dirty();
        }
    });
}
