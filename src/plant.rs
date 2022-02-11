use bevy::{prelude::*, utils::HashMap};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial};
use dcc_lsystem::{ArenaId, LSystem, LSystemBuilder};
use regex::Regex;

use crate::{events::GameEvent, ui::OptionsComponent};

pub struct PlantPlugin;

impl Plugin for PlantPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GameEvent>()
            .add_startup_system(setup_system)
            .add_system(solver_system);
    }
}

#[derive(Component)]
pub struct PlantComponent {
    pub structure: LSystem,
    pub action_map: HashMap<char, Action>,
}

impl PlantComponent {
    pub fn render_actions(&self) -> Vec<Action> {
        self.structure
            .render()
            .chars()
            .map(|c| *self.action_map.get(&c).unwrap())
            .collect()
    }
}

#[derive(Component, Default)]
pub struct PlantInfoComponent {
    pub line_count: usize,
}

#[derive(Component, Default)]
pub struct PlantRendererComponent {
    state: RenderState,
}

#[derive(Default)]
struct RenderState {
    pub cursor: (Vec3, Quat),
    states: Vec<(Vec3, Quat)>,
}

type PosRot = (Vec3, Quat);

impl RenderState {
    pub fn push(&mut self, position: Vec3, rotation: Quat) {
        self.states.push((position, rotation));
    }
    pub fn pop(&mut self) -> Option<PosRot> {
        self.states.pop()
    }
}

impl PlantRendererComponent {
    pub fn generate_lines(
        &mut self,
        actions: &[Action],
        options: &OptionsComponent,
    ) -> Vec<Vec<Vec3>> {
        let (mut pos, mut rot) = self.state.cursor;

        let mut lines = vec![];
        let mut verts = vec![];

        for action in actions {
            match action {
                Action::Nothing => {
                    verts.push(pos);
                }
                Action::Forwards => {
                    pos += (rot * Vec3::Y) * options.segment_length;
                    verts.push(pos);
                }
                Action::Rotate(r) => {
                    let angle = options.rotation_amount.to_radians();
                    let params = match r {
                        Direction::XPos => (angle, 0f32, 0f32),
                        Direction::XNeg => (-angle, 0f32, 0f32),
                        Direction::YPos => (0f32, angle, 0f32),
                        Direction::YNeg => (0f32, -angle, 0f32),
                        Direction::ZPos => (0f32, 0f32, angle),
                        Direction::ZNeg => (0f32, 0f32, -angle),
                    };

                    rot *= Quat::from_euler(EulerRot::XYZ, params.0, params.1, params.2);
                }
                Action::Push => {
                    self.state.push(pos, rot);
                }
                Action::Pop => {
                    if let Some((new_pos, new_rot)) = self.state.pop() {
                        pos = new_pos;
                        rot = new_rot;

                        // additionally, push the verts to our line. this is due to how polyline works
                        // HOWEVER these individual lines also are very expensive. We can disable this in our UI
                        if options.expensive_rendering {
                            lines.push(verts.drain(0..).collect());
                        } else {
                            // Thanks to @aevyrie as usual:
                            // https://github.com/ForesightMiningSoftwareCorporation/bevy_polyline/issues/20#issuecomment-1035624250
                            verts.push(Vec3::splat(f32::NEG_INFINITY));
                        }
                        verts.push(pos);
                    }
                }
            }
        }
        lines.push(verts.drain(0..).collect());
        lines
    }
}

fn solver_system(
    mut cmd: Commands,
    mut plants: Query<
        (
            Entity,
            &mut PlantComponent,
            &mut PlantInfoComponent,
            &mut PlantRendererComponent,
            &OptionsComponent,
        ),
        Changed<PlantComponent>,
    >,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
) {
    plants.for_each_mut(|(e, mut plant, mut info, mut render, options)| {
        cmd.entity(e).despawn_descendants();

        plant.structure.step_by(options.iterations);
        let instructions = plant.render_actions();
        // build lines
        let mut lines: Vec<Vec<Vec3>> = render.generate_lines(&instructions, options);
        info.line_count = lines.len();

        cmd.entity(e).with_children(|c| {
            let lines_len = lines.len();
            for (i, line) in lines.drain(0..).enumerate() {
                let normalized = i as f32 / lines_len as f32;
                c.spawn_bundle(PolylineBundle {
                    polyline: polylines.add(Polyline { vertices: line }),
                    material: polyline_materials.add(PolylineMaterial {
                        width: 20.0 - (normalized * 10.0),
                        color: get_color(normalized),
                        perspective: true,
                    }),
                    ..Default::default()
                });
            }
        });
    });
}

#[inline(always)]
fn get_color(percent: f32) -> Color {
    Color::rgb(0., 1.0 - (percent * 0.3), 0.1 + (percent * 0.2))
}

#[derive(Clone, Copy)]
pub enum Action {
    Nothing,
    Forwards,
    Rotate(Direction),
    Push,
    Pop,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Action::Nothing => {
                    "Do nothing".to_string()
                }
                Action::Forwards => {
                    "Move forwards".to_string()
                }
                Action::Rotate(direction) => {
                    format!("Rotate {direction}")
                }
                Action::Push => {
                    "Push transform".to_string()
                }
                Action::Pop => {
                    "Pop transform".to_string()
                }
            }
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::XPos => "right",
                Self::XNeg => "left",
                Self::YPos => "forwards",
                Self::YNeg => "back",
                Self::ZPos => "up",
                Self::ZNeg => "down",
            }
        )
    }
}

fn setup_system(mut cmd: Commands, mut events: EventWriter<GameEvent>) {
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
        .spawn()
        .insert(plant)
        .insert(builder)
        .insert(OptionsComponent::default())
        .insert(PlantRendererComponent::default())
        .insert(PlantInfoComponent::default())
        .id();
    events.send(GameEvent::TriggerUpdate(entity));
}

#[derive(Component, Default)]
pub struct PlantBuilderComponent {
    builder: LSystemBuilder,
    tokens: HashMap<char, (ArenaId, Action)>,
}

impl PlantBuilderComponent {
    /// Add a transformation rule to the builder.
    /// Panics if a necessary token is not found
    pub fn add_rule<S>(&mut self, rule: S) -> anyhow::Result<&mut Self>
    where
        S: AsRef<str>,
    {
        let rule = rule.as_ref();

        lazy_static::lazy_static! {
            static ref RE: Regex = Regex::new(r"\s*(\w)\s*=\s*((?:\s*\S+\s*)*)\s*").unwrap();
        }

        let cap = RE
            .captures(rule)
            .ok_or_else(|| anyhow::anyhow!("Captures error: {rule}"))?;

        // The LHS of our rule
        let lhs = self.get_token(cap[1].chars().next().unwrap())?.0;

        // Construct the RHS of our rule
        let mut rule = Vec::new();

        for token in cap[2].chars() {
            let token = self.get_token(token)?;
            rule.push(token.0);
        }

        // Add the rule to our builder
        self.builder.transformation_rule(lhs, rule).ok();
        Ok(self)
    }

    pub fn set_rules<S>(&mut self, rules: &[S]) -> anyhow::Result<&mut Self>
    where
        S: AsRef<str>,
    {
        self.builder.rules.clear();
        for rule in rules {
            self.add_rule(rule)?;
        }
        Ok(self)
    }

    pub fn add_token(&mut self, token: char, action: Action) -> &mut Self {
        self.tokens
            .insert(token, (self.builder.token(token).unwrap(), action));
        self
    }

    pub fn set_tokens(&mut self, tokens: &[(char, Action)]) -> &mut Self {
        self.builder = Default::default();
        self.tokens = Default::default();
        for (token, action) in tokens {
            self.add_token(*token, *action);
        }
        self
    }

    pub fn set_axiom(&mut self, tokens: impl AsRef<str>) -> anyhow::Result<&mut Self> {
        let tokens: Vec<ArenaId> = tokens
            .as_ref()
            .chars()
            .filter_map(|token| self.get_token(token).map(|(id, _)| id).ok())
            .collect();
        self.builder.axiom(tokens).ok();
        Ok(self)
    }

    pub fn get_token(&self, token: char) -> anyhow::Result<(ArenaId, Action)> {
        Ok(*self
            .tokens
            .get(&token)
            .ok_or_else(|| anyhow::anyhow!("Could not get token with name {token}"))?)
    }

    pub fn generate(&self) -> PlantComponent {
        let f = self.builder.clone();
        PlantComponent {
            structure: f.finish().unwrap(),
            action_map: self.tokens.iter().map(|(c, (_, a))| (*c, *a)).collect(),
        }
    }
}
