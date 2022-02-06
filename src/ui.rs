use bevy::prelude::*;
use bevy_egui::egui;

use crate::plant::*;
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(ui_system);
    }
}

fn ui_system(
    mut ctx: ResMut<bevy_egui::EguiContext>,
    mut plants: Query<(&mut RulesComponent, &mut PlantRendererComponent), With<PlantComponent>>,
) {
    let mut i = 0;
    plants.for_each_mut(|(mut rules, mut render)| {
        i += 1;
        let mut rules_dirty = false;
        let mut render_dirty = false;
        egui::Window::new(format!("Plant {i} settings"))
            .collapsible(false)
            .default_pos(egui::Pos2::new(32., 32.))
            .resizable(false)
            .show(ctx.ctx_mut(), |ui| {
                ui.style_mut().spacing.slider_width = 300.;
                // Show render settings
                ui.label("Iterations");
                let iterations = ui.add(egui::Slider::new(&mut render.options.iterations, 1..=9));

                ui.separator();

                ui.label("Rotation angle");
                let rot_angle = ui.add(
                    egui::Slider::new(&mut render.options.rotation_angle, 0f32..=180f32)
                        .max_decimals(2)
                        .smart_aim(false),
                );

                ui.separator();

                ui.label("Segment length");
                let segment_length = ui.add(egui::Slider::new(
                    &mut render.options.segment_length,
                    0.01f32..=0.2f32,
                ));

                render_dirty =
                    rot_angle.changed() || segment_length.changed() || iterations.changed();

                ui.separator();

                // Show rules text
                ui.label("Rules");
                for rule in rules.iter_mut() {
                    ui.vertical(|ui| {
                        let text = ui.add(
                            egui::TextEdit::multiline(rule)
                                .code_editor()
                                .desired_rows(1)
                                .desired_width(f32::INFINITY),
                        );
                        let remove_button = ui.button("Remove rule");
                        if text.changed() || remove_button.clicked() {
                            rules_dirty = true;
                        }
                    });
                    ui.separator();
                }
                if ui.button("Add rule").clicked() {
                    rules.push(Default::default());
                    rules_dirty = true;
                }
            });
        if rules_dirty {
            rules.dirty();
        }
        if render_dirty {
            render.dirty();
        }
    });
}
