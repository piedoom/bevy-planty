use std::{
    ops::DerefMut,
    slice::{Iter, IterMut},
};

use bevy::{prelude::*, utils::HashMap};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial};
use dcc_lsystem::{ArenaId, LSystem, LSystemBuilder};
use lazy_static::__Deref;
use regex::Regex;

pub struct PlantPlugin;

impl Plugin for PlantPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_system)
            .add_system(solver_system)
            .add_system(update_dirty_system)
            .add_system(update_plants_system);
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

#[derive(Component)]
pub struct PlantRendererComponent {
    state: RenderState,
    pub options: RenderOptions,
    dirty: bool,
}

impl Default for PlantRendererComponent {
    fn default() -> Self {
        Self {
            state: Default::default(),
            options: Default::default(),
            dirty: true,
        }
    }
}

impl PlantRendererComponent {
    pub fn dirty(&mut self) {
        self.dirty = true;
    }
}

#[derive(Component)]
pub struct RenderOptions {
    /// Length of each segment
    pub segment_length: f32,
    /// Angle IN DEGREES, NOT RADIANS for rotations
    pub rotation_angle: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            segment_length: 0.1f32,
            rotation_angle: 20f32,
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
                    let angle = self.options.rotation_angle.to_radians()
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
        lines
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
        cmd.entity(e).despawn_descendants();

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
        .set_tokens(&[
            ('X', Action::Nothing),
            ('F', Action::Forwards),
            ('+', Action::Rotate(Direction::Right)),
            ('-', Action::Rotate(Direction::Left)),
            ('[', Action::Push),
            (']', Action::Pop),
        ])
        .set_axiom(&['X'])
        .unwrap();

    let plant = builder.generate();
    cmd.spawn()
        .insert(plant)
        .insert(builder)
        .insert(RulesComponent::new(&["X=F+[[X]-X]-F[-FX]+X", "F=FF"]))
        .insert(PlantRendererComponent::default());
}

#[derive(Component, Default)]
pub struct PlantBuilderComponent {
    builder: LSystemBuilder,
    tokens: HashMap<char, (ArenaId, Action)>,
}

impl PlantBuilderComponent {
    /// Add a transformation rule to the builder.
    /// Panics if a necessary token is not found
    pub fn add_rule<'a, S: Into<&'a str>>(&mut self, rule: S) -> anyhow::Result<&mut Self> {
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
        self.builder.transformation_rule(lhs, rule).ok();
        Ok(self)
    }

    pub fn set_rules<'a, S: Into<&'a str> + Clone>(
        &mut self,
        rules: &[S],
    ) -> anyhow::Result<&mut Self> {
        self.builder.rules.clear();
        for rule in rules {
            let s: &'a str = (rule.clone()).into();
            self.add_rule(s)?;
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

    pub fn set_axiom(&mut self, tokens: &[char]) -> anyhow::Result<&mut Self> {
        let tokens: Vec<ArenaId> = tokens
            .iter()
            .map(|token| {
                self.get_token(*token)
                    .expect("Axiom token {token} not found")
                    .0
            })
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

#[derive(Component)]
pub struct RulesComponent {
    rules: Vec<String>,
    dirty: bool,
}

impl RulesComponent {
    pub fn new(rules: &[&str]) -> Self {
        Self {
            rules: rules.iter().map(|x| String::from(*x)).collect(),
            dirty: true,
        }
    }

    pub fn dirty(&mut self) {
        self.dirty = true
    }
}

impl From<Vec<String>> for RulesComponent {
    fn from(v: Vec<String>) -> Self {
        Self {
            rules: v,
            dirty: true,
        }
    }
}

impl std::ops::Deref for RulesComponent {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.rules
    }
}

impl std::ops::DerefMut for RulesComponent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rules
    }
}

fn update_dirty_system(
    mut settings: Query<(Entity, &mut RulesComponent, &mut PlantRendererComponent)>,
    mut builders: Query<&mut PlantBuilderComponent>,
) {
    settings.for_each_mut(|(entity, mut rules, mut render)| {
        // As we access this mutably, it is always marked as "changed"
        // So, we need to manually mark as dirty when we actually change the value and cannot rely on Changed
        if rules.dirty {
            let new_rules: Vec<_> = rules.iter().map(String::as_str).collect();
            if let Ok(mut builder) = builders.get_mut(entity) {
                builder.set_rules(&new_rules).ok();
            }
            rules.dirty = false;
        }

        if render.dirty {
            // Mark as changed manually
            if let Ok(mut builder) = builders.get_mut(entity) {
                builder.set_changed();
            }

            render.dirty = false;
        }
    });
}

fn update_plants_system(
    mut plants: Query<
        (&PlantBuilderComponent, &mut PlantComponent),
        Changed<PlantBuilderComponent>,
    >,
) {
    if !plants.is_empty() {
        let len = plants.iter().count();
        info!("Redraw requested for {} entities", len);
    }
    plants.for_each_mut(|(builder, mut plant)| {
        *plant = builder.generate();
    });
}
