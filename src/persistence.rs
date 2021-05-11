use crate::model::*;
use bevy::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PersistenceEvent<R> {
    FileOut,
    FileIn,
    _Phantom((std::marker::PhantomData<R>, std::convert::Infallible)),
}

pub struct PersistencePlugin<R> {
    phantom: std::marker::PhantomData<R>,
}

impl<R> Plugin for PersistencePlugin<R>
where
    R: 'static + Persistable,
{
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_event::<PersistenceEvent<R>>()
            .add_system(Self::handler.system());
    }
}

use std::fs::File;
use std::path::Path;

fn file_path() -> &'static Path {
    Path::new("slides.json")
}

pub trait Persistable: Clone + Send + Sync + Serialize + DeserializeOwned {}
impl<T> Persistable for T where T: Clone + Send + Sync + Serialize + DeserializeOwned {}

impl<R> PersistencePlugin<R>
where
    R: 'static + Persistable,
{
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData::default(),
        }
    }

    fn handler(
        mut events: EventReader<PersistenceEvent<R>>,
        resources: Query<(Entity, &R)>,
        mut commands: Commands,
    ) {
        for e in events.iter() {
            match e {
                PersistenceEvent::FileIn => {
                    for (se, _res) in resources.iter() {
                        commands.entity(se).despawn();
                    }

                    if file_path().exists() {
                        info!("File exists, loading!");
                        let f = File::open(file_path()).expect("Failed to read resources file");
                        let resources: Vec<R> =
                            serde_json::from_reader(f).expect("Failed to parse resources file");
                        for res in resources.iter() {
                            commands.spawn().insert(res.clone());
                        }
                        info!("Loaded {} resources", resources.len());
                    } else {
                        warn!("File does not exist");
                    }
                }
                PersistenceEvent::FileOut => {
                    info!("Writing to file!");
                    let resources: Vec<R> = resources.iter().map(|(_, res)| res).cloned().collect();
                    let f = File::create(file_path()).expect("Failed to write to resources file");
                    serde_json::to_writer_pretty(f, &resources)
                        .expect("Failed to rialize to resources files");
                    info!("Wrote {} resources", resources.len());
                }
                PersistenceEvent::_Phantom(_) => unreachable!(),
            }
        }
    }
}
