use bevy::prelude::*;

use crate::{
    plant::{PlantBuilderComponent, PlantComponent},
    ui::OptionsComponent,
};

pub enum GameEvent {
    TriggerUpdate(Entity),
}

pub(crate) fn process_events_system(
    mut events: EventReader<GameEvent>,
    mut plants: Query<(
        &OptionsComponent,
        &mut PlantBuilderComponent,
        &mut PlantComponent,
    )>,
) {
    for event in events.iter() {
        match event {
            GameEvent::TriggerUpdate(entity) => {
                if !plants.is_empty() {
                    let len = plants.iter().count();
                    info!("Redraw requested for {} entities", len);
                }
                if let Ok((options, mut builder, mut plant)) = plants.get_mut(*entity) {
                    // set builder to match options
                    builder.set_axiom(&options.axiom).ok();
                    builder.set_rules(&options.rules).ok();
                    *plant = builder.generate();
                }
            }
        }
    }
}
