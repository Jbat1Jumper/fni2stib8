use crate::model::*;
use crate::persistence::PersistenceEvent;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

pub fn slide_list(
    egui_context: ResMut<EguiContext>,
    slides: Query<(Entity, &Slide)>,
    mut fs_event: EventWriter<PersistenceEvent>,
    mut commands: Commands,
) {
    egui::Window::new("Slides").show(egui_context.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Add new").clicked() {
                commands.insert_resource(AddSlidePrompt::default());
            }
            if ui.button("File In").clicked() {
                fs_event.send(PersistenceEvent::FileIn);
            }
            if ui.button("File Out").clicked() {
                fs_event.send(PersistenceEvent::FileOut);
            }
        });

        ui.separator();

        for (e, v) in slides.iter() {
            ui.horizontal(|ui| {
                ui.label(format!("{}", v.name,));
                if ui.small_button("edit").clicked() {
                    commands.spawn().insert(SlideEditor::new_for(e, v));
                }
                if ui.small_button("remove").clicked() {
                    commands.entity(e).despawn();
                }
            });
        }
    });
}

pub struct SlideEditor {
    pub target: Entity,
    pub unsaved: Slide,
    pub info: String,
}

impl SlideEditor {
    pub fn new_for(target: Entity, slide: &Slide) -> Self {
        Self {
            target,
            unsaved: slide.clone(),
            info: String::new(),
        }
    }
    pub fn render(
        egui_context: ResMut<EguiContext>,
        mut editors: Query<(Entity, &mut Self)>,
        slides: Query<(Entity, &Slide)>,
        mut commands: Commands,
    ) {
        for (eid, mut e) in editors.iter_mut() {
            let saved = match slides.get(e.target) {
                Err(_) => {
                    commands.entity(eid).despawn();
                    continue;
                }
                Ok((_, sc)) => sc,
            };
            let title = {
                if e.unsaved == *saved {
                    format!("{}", saved.name)
                } else {
                    format!("{} (unsaved)", saved.name)
                }
            };
            let mut open = true;
            egui::Window::new(title)
                .id(egui::Id::new(eid))
                .open(&mut open)
                .show(egui_context.ctx(), |ui| {
                    ui.text_edit_singleline(&mut e.unsaved.name);
                    ui.text_edit_multiline(&mut e.unsaved.description);
                    if ui.button("Save").clicked() {
                        if e.unsaved.name.is_empty() {
                            e.info = "Name can not be empty".into();
                            return;
                        }

                        if slides
                            .iter()
                            .any(|(en, v)| v.name == e.unsaved.name && en != e.target)
                        {
                            e.info = "Name already taken".into();
                            return;
                        }

                        commands.entity(e.target).insert(e.unsaved.clone());
                        e.info = "Saved sucessfully".into();
                    }

                    ui.separator();
                    ui.label(&e.info);
                });
            if !open {
                commands.entity(eid).despawn();
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct AddSlidePrompt {
    pub name: String,
    pub info: String,
}

impl AddSlidePrompt {
    pub fn render(
        egui_context: ResMut<EguiContext>,
        prompt: Option<ResMut<Self>>,
        mut commands: Commands,
        slides: Query<&Slide>,
    ) {
        if let Some(mut prompt) = prompt {
            egui::Window::new("Create new Slide").show(egui_context.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name: ");
                    ui.text_edit_singleline(&mut prompt.name);
                });
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        commands.remove_resource::<AddSlidePrompt>();
                    }
                    if ui.button("Create").clicked() {
                        if prompt.name.is_empty() {
                            prompt.info = "Name can not be empty".into();
                            return;
                        }

                        if slides.iter().any(|v| v.name == prompt.name) {
                            prompt.info = "Name already taken".into();
                            return;
                        }

                        let slide = Slide::new(prompt.name.clone());
                        let t = commands.spawn().insert(slide.clone()).id();
                        commands.spawn().insert(SlideEditor::new_for(t, &slide));
                        commands.remove_resource::<AddSlidePrompt>();
                    }
                });
                ui.separator();
                ui.label(&prompt.info);
            });
        }
    }
}
