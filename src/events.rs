use bevy::{app::Events, prelude::*};

use crate::{
    plant::{
        Action, Direction, PlantBuilderComponent, PlantBundle, PlantComponent,
        PlantRendererComponent, PlantStatsComponent, SelectedPlantsResource,
    },
    ui::OptionsComponent,
};

pub enum GameEvent {
    TriggerUpdate(Entity),
    SpawnNew(Transform),
    RemoveToken {
        entity: Entity,
        token: char,
    },
    ChangeToken {
        entity: Entity,
        prev: char,
        next: char,
    },
    ChangeAction {
        entity: Entity,
        token: char,
        action: Action,
    },
    AddToken {
        entity: Entity,
        token: char,
        action: Action,
    },
}

pub(crate) fn process_events_system(
    mut cmd: Commands,
    mut events: ResMut<Events<GameEvent>>,
    mut selected: ResMut<SelectedPlantsResource>,
    mut plants: Query<(
        &OptionsComponent,
        &mut PlantBuilderComponent,
        &mut PlantComponent,
    )>,
) {
    let mut events_buf = vec![];
    for event in events.drain() {
        match event {
            GameEvent::TriggerUpdate(entity) => {
                if !plants.is_empty() {
                    let len = plants.iter().count();
                    info!("Redraw requested for {} entities", len);
                }
                if let Ok((options, mut builder, mut plant)) = plants.get_mut(entity) {
                    // set builder to match options
                    builder.set_axiom(&options.axiom).ok();
                    builder.set_rules(&options.rules).ok();
                    *plant = builder.generate();
                }
            }
            GameEvent::SpawnNew(transform) => {
                let mut builder = PlantBuilderComponent::default();
                builder
                    .set_tokens(&[
                        ('X', Action::Nothing),
                        ('F', Action::Forwards),
                        ('+', Action::Rotate(Direction::XPos)),
                        ('-', Action::Rotate(Direction::XNeg)),
                        ('>', Action::Rotate(Direction::YPos)),
                        ('<', Action::Rotate(Direction::YNeg)),
                        ('^', Action::Rotate(Direction::ZPos)),
                        ('v', Action::Rotate(Direction::ZNeg)),
                        ('[', Action::Push),
                        (']', Action::Pop),
                    ])
                    .set_axiom("X")
                    .ok();

                let plant = builder.generate();
                let entity = cmd
                    .spawn_bundle(PlantBundle {
                        plant,
                        builder,
                        options: OptionsComponent::default(),
                        renderer: PlantRendererComponent::default(),
                        stats: PlantStatsComponent::default(),
                        transform,
                        global_transform: transform.into(),
                    })
                    .id();
                events_buf.push(GameEvent::TriggerUpdate(entity));
                selected.0.insert(entity, ());
            }
            GameEvent::RemoveToken { token, entity } => {
                if let Ok((options, mut builder, mut plant)) = plants.get_mut(entity) {
                    builder.remove_token(token);
                    events_buf.push(GameEvent::TriggerUpdate(entity));
                }
            }
            GameEvent::ChangeToken { entity, prev, next } => {
                if let Ok((options, mut builder, mut plant)) = plants.get_mut(entity) {
                    if let Some((arena_id, action)) = builder.remove_token(prev) {
                        builder.add_token(next, action);
                        events_buf.push(GameEvent::TriggerUpdate(entity));
                    }
                }
            }
            GameEvent::ChangeAction {
                entity,
                token,
                action,
            } => {
                if let Ok((options, mut builder, mut plant)) = plants.get_mut(entity) {
                    if let Some((_, _)) = builder.remove_token(token) {
                        builder.add_token(token, action);
                        events_buf.push(GameEvent::TriggerUpdate(entity));
                    }
                }
            }
            GameEvent::AddToken {
                entity,
                token,
                action,
            } => {
                if let Ok((_, mut builder, ..)) = plants.get_mut(entity) {
                    builder.add_token(token, action);
                    events_buf.push(GameEvent::TriggerUpdate(entity));
                }
            }
        }
    }
    for event in events_buf {
        events.send(event);
    }
}
