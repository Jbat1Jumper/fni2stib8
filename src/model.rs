use bevy::prelude::*;
use serde::*;
use serde_json;

pub struct ModelPlugin;

impl Plugin for ModelPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_event::<SlideEvent>()
            .add_system(event_handler.system());
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub name: String,
    pub description: String,
    pub actions: Vec<Action>,
}

impl Slide {
    pub fn new(name: String) -> Self {
        Self {
            name,
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum SlideEvent {
    Created(Slide),
    Updated(Slide),
    Renamed(String, String),
    Deleted(String),
}

fn event_handler(
    mut events: EventReader<SlideEvent>,
    mut query: Query<(Entity, &mut Slide)>,
    mut commands: Commands,
) {
    for e in events.iter() {
        info!("{:?}", e);
        match e {
            SlideEvent::Created(slide) => {
                commands.spawn().insert(slide.clone());
            }
            SlideEvent::Updated(slide) => {
                for (_, mut s) in query.iter_mut() {
                    if s.name == slide.name {
                        *s = slide.clone();
                    }
                }
            }
            SlideEvent::Renamed(old_name, new_name) => {
                for (_, mut s) in query.iter_mut() {
                    if s.name == *old_name {
                        s.name = new_name.clone();
                    }
                    for mut a in s.actions.iter_mut() {
                        if a.target_slide == *old_name {
                            a.target_slide = new_name.clone();
                        }
                    }
                }
            }
            SlideEvent::Deleted(name) => {
                for (eid, s) in query.iter_mut() {
                    if s.name == *name {
                        commands.entity(eid).despawn();
                    }
                }
            }
        }
    }
}
