use std::ops::{Deref, DerefMut};

use bevy::{prelude::*, utils::HashMap};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial};
use dcc_lsystem::{ArenaId, LSystem, LSystemBuilder};
use regex::Regex;

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
pub struct PlantRendererComponent {
    state: RenderState,
    pub options: RenderOptions,
}

#[derive(Component)]
pub struct RenderOptions {
    /// Length of each segment
    segment_length: f32,
    /// Angle in radians for rotations
    rotation_angle: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            segment_length: 1.5f32,
            rotation_angle: 20f32.to_radians(),
        }
    }
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
    pub fn generate_lines(&mut self, actions: &[Action]) -> Vec<Vec<Vec3>> {
        let (mut pos, mut rot) = self.state.cursor;

        let mut lines = vec![];
        let mut verts = vec![];

        for action in actions {
            match action {
                Action::Nothing => {
                    verts.push(pos);
                }
                Action::Forwards => {
                    pos += (rot * Vec3::Y) * self.options.segment_length;
                    verts.push(pos);
                }
                Action::Rotate(r) => {
                    let angle = self.options.rotation_angle
                        * if r == &Direction::Left { -1f32 } else { 1f32 };
                    rot *= Quat::from_euler(EulerRot::XYZ, angle * 2f32, angle, 0f32);
                }
                Action::Push => {
                    self.state.push(pos, rot);
                }
                Action::Pop => {
                    if let Some((new_pos, new_rot)) = self.state.pop() {
                        pos = new_pos;
                        rot = new_rot;
                    }
                    // additionally, push the verts to our line. this is due to how polyline works
                    lines.push(verts.drain(0..).collect());
                    verts.push(pos);
                }
            }
        }
        info!("{:?}", &verts);
        lines
    }
}

#[derive(Component, Default)]
pub struct PlantBuilderComponent {
    pub builder: LSystemBuilder,
    pub tokens: HashMap<char, (ArenaId, Action)>,
}
pub struct PlantPlugin;

impl Plugin for PlantPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_system)
            .add_system(solver_system);
    }
}

fn solver_system(
    mut cmd: Commands,
    mut plants: Query<
        (Entity, &mut PlantComponent, &mut PlantRendererComponent),
        Changed<PlantComponent>,
    >,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
) {
    plants.for_each_mut(|(e, mut plant, mut render)| {
        // system.step_by() applies our production rule a number of times
        plant.structure.step_by(5);
        let instructions = plant.render_actions();
        // build lines
        let lines: Vec<Vec<Vec3>> = render.generate_lines(&instructions);

        cmd.entity(e).with_children(|c| {
            for line in lines {
                c.spawn_bundle(PolylineBundle {
                    polyline: polylines.add(Polyline { vertices: line }),
                    material: polyline_materials.add(PolylineMaterial {
                        width: 3.0,
                        color: Color::GREEN,
                        perspective: false,
                    }),
                    ..Default::default()
                });
            }
        });
    });
}

#[derive(Clone, Copy)]
pub enum Action {
    Nothing,
    Forwards,
    Rotate(Direction),
    Push,
    Pop,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Right,
    Left,
}

fn setup_system(mut cmd: Commands) {
    let mut builder = PlantBuilderComponent::default();
    builder
        .tokens(&[
            ('X', Action::Nothing),
            ('F', Action::Forwards),
            ('+', Action::Rotate(Direction::Right)),
            ('-', Action::Rotate(Direction::Left)),
            ('[', Action::Push),
            (']', Action::Pop),
        ])
        .axiom(&['X'])
        .unwrap()
        .rules(&["X=F+[[X]-X]-F[-FX]+X", "F=FF"])
        .unwrap();

    let plant = builder.finish();
    cmd.spawn()
        .insert(plant)
        .insert(PlantRendererComponent::default());
}

impl PlantBuilderComponent {
    /// Add a transformation rule to the builder.
    /// Panics if a necessary token is not found
    pub fn rule<'a, S: Into<&'a str>>(&mut self, rule: S) -> anyhow::Result<&mut Self> {
        let rule = rule.into();

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
        self.builder.transformation_rule(lhs, rule);
        Ok(self)
    }

    pub fn rules<'a, S: Into<&'a str> + Copy>(&mut self, rules: &[S]) -> anyhow::Result<&mut Self> {
        for rule in rules {
            let s: &'a str = (*rule).into();
            self.rule(s)?;
        }
        Ok(self)
    }

    pub fn token(&mut self, token: char, action: Action) -> &mut Self {
        self.tokens
            .insert(token, (self.builder.token(token), action));
        self
    }

    pub fn tokens(&mut self, tokens: &[(char, Action)]) -> &mut Self {
        for (token, action) in tokens {
            self.token(*token, *action);
        }
        self
    }

    pub fn axiom(&mut self, tokens: &[char]) -> anyhow::Result<&mut Self> {
        let tokens: Vec<ArenaId> = tokens
            .iter()
            .map(|token| {
                self.get_token(*token)
                    .expect("Axiom token {token} not found")
                    .0
            })
            .collect();
        self.builder.axiom(tokens);
        Ok(self)
    }

    pub fn get_token(&self, token: char) -> anyhow::Result<(ArenaId, Action)> {
        Ok(*self
            .tokens
            .get(&token)
            .ok_or_else(|| anyhow::anyhow!("Could not get token with name {token}"))?)
    }

    pub fn finish(&mut self) -> PlantComponent {
        let f = self.builder.clone();
        PlantComponent {
            structure: f.finish(),
            action_map: self.tokens.iter().map(|(c, (_, a))| (*c, *a)).collect(),
        }
    }
}
