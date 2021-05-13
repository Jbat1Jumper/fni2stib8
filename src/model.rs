use std::path::Path;

use bevy::prelude::*;
use serde::*;
use serde_json;

use crate::{images::Background, persistence::Persistable};

pub struct ModelPlugin;

impl Plugin for ModelPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_plugin(CrudPlugin::<Slide>::new())
            .add_system(update_references.system())
            .add_system(update_references_to_backgrounds.system());
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub background: String,
    pub actions: Vec<Action>,
}

impl Persistable for Slide {
    fn file_path() -> &'static Path {
        Path::new("slides.json")
    }
}

impl Slide {
    pub fn new(name: String) -> Self {
        Self {
            name,
            background: "".into(),
            description: String::new(),
            actions: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
pub struct Action {
    pub text: String,
    pub target_slide: String,
}

pub struct CrudPlugin<R> {
    _phantom: std::marker::PhantomData<R>,
}

impl<R: 'static + Crudable> Plugin for CrudPlugin<R> {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_event::<CrudEvent<R>>()
            .add_system(Self::event_handler.system());
    }
}

pub trait Crudable: Clone + Send + Sync + std::fmt::Debug {
    fn name(&self) -> String;
    fn set_name(&mut self, new_name: String);
    fn default_name_prefix() -> &'static str;
}

impl Crudable for Slide {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn set_name(&mut self, new_name: String) {
        self.name = new_name;
    }
    fn default_name_prefix() -> &'static str {
        "slide"
    }
}

fn update_references_to_backgrounds(
    mut events: EventReader<CrudEvent<Background>>,
    mut query: Query<(Entity, &mut Slide)>,
) {
    for e in events.iter() {
        match e {
            CrudEvent::Renamed(old_name, new_name) => {
                for (_, mut s) in query.iter_mut() {
                    if s.background == *old_name {
                        s.background = new_name.clone();
                    }
                }
            }
            _ => {}
        }
    }
}

fn update_references(
    mut events: EventReader<CrudEvent<Slide>>,
    mut query: Query<(Entity, &mut Slide)>,
) {
    for e in events.iter() {
        match e {
            CrudEvent::Renamed(old_name, new_name) => {
                for (_, mut s) in query.iter_mut() {
                    for mut a in s.actions.iter_mut() {
                        if a.target_slide == *old_name {
                            a.target_slide = new_name.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl<R: 'static + Crudable> CrudPlugin<R> {
    pub fn new() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
    fn event_handler(
        mut events: EventReader<CrudEvent<R>>,
        mut query: Query<(Entity, &mut R)>,
        mut commands: Commands,
    ) {
        for e in events.iter() {
            info!("{:?}", e);
            match e {
                CrudEvent::Created(res) => {
                    commands.spawn().insert(res.clone());
                }
                CrudEvent::Updated(res) => {
                    for (_, mut s) in query.iter_mut() {
                        if s.name() == res.name() {
                            *s = res.clone();
                        }
                    }
                }
                CrudEvent::Renamed(old_name, new_name) => {
                    for (_, mut s) in query.iter_mut() {
                        if s.name() == *old_name {
                            s.set_name(new_name.clone());
                        }
                    }
                }
                CrudEvent::Deleted(name) => {
                    for (eid, s) in query.iter_mut() {
                        if s.name() == *name {
                            commands.entity(eid).despawn();
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CrudEvent<R> {
    Created(R),
    Updated(R),
    Renamed(String, String),
    Deleted(String),
}
