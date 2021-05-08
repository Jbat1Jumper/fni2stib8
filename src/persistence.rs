use crate::model::*;
use bevy::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PersistenceEvent {
    FileOut,
    FileIn,
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_event::<PersistenceEvent>()
            .add_system(handler.system());
    }
}

use std::fs::File;
use std::path::Path;

fn file_path() -> &'static Path {
    Path::new("slides.json")
}

fn handler(
    mut events: EventReader<PersistenceEvent>,
    slides: Query<(Entity, &Slide)>,
    mut commands: Commands,
) {
    for e in events.iter() {
        match e {
            PersistenceEvent::FileIn => {
                for (se, _slide) in slides.iter() {
                    commands.entity(se).despawn();
                }

                if file_path().exists() {
                    info!("File exists, loading!");
                    let f = File::open(file_path()).expect("Failed to read slides file");
                    let slides: Vec<Slide> =
                        serde_json::from_reader(f).expect("Failed to parse slides file");
                    for slide in slides.iter() {
                        commands.spawn().insert(slide.clone());
                    }
                    info!("Loaded {} slides", slides.len());
                } else {
                    warn!("File does not exist");
                }
            }
            PersistenceEvent::FileOut => {
                info!("Writing to file!");
                let slides: Vec<Slide> = slides.iter().map(|(_, slide)| slide).cloned().collect();
                let f = File::create(file_path()).expect("Failed to write to slides file");
                serde_json::to_writer(f, &slides).expect("Failed to rialize to slides files");
                info!("Wrote {} slides", slides.len());
            }
        }
    }
}
