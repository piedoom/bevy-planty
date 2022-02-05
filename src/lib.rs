use bevy::prelude::PluginGroup;

mod error;
mod plant;

pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(plant::PlantPlugin);
    }
}
