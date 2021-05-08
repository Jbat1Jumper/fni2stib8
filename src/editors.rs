use crate::model::*;
use crate::persistence::PersistenceEvent;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, ScrollArea},
    EguiContext,
};

pub struct EditorsPlugin;

impl Plugin for EditorsPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_system(AddSlidePrompt::render.system())
            .add_system(SlideEditor::render.system())
            .add_system(PersistConfirmationDialog::render.system())
            .add_system(slide_list.system());
    }
}

struct PersistConfirmationDialog(PersistenceEvent);

impl PersistConfirmationDialog {
    fn render(
        egui_context: ResMut<EguiContext>,
        mut commands: Commands,
        dialog: Option<Res<Self>>,
        mut fs_event: EventWriter<PersistenceEvent>,
    ) {
        if dialog.is_none() {
            return;
        }
        let dialog = dialog.unwrap();

        egui::Window::new("Please confirm").show(egui_context.ctx(), |ui| {
            ui.label(match dialog.0 {
                PersistenceEvent::FileIn => "Doing a File In will erase all your unsaved changes.",
                PersistenceEvent::FileOut => "Doing a File Out will override the file",
            });
            ui.horizontal(|ui| {
                if ui.button("Proceed").clicked() {
                    fs_event.send(dialog.0);
                    commands.remove_resource::<Self>();
                }
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<Self>();
                }
            });
        });
    }
}

fn slide_list(
    egui_context: ResMut<EguiContext>,
    slides: Query<(Entity, &Slide)>,
    mut commands: Commands,
) {
    egui::Window::new("Slides").show(egui_context.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Add new").clicked() {
                commands.insert_resource(AddSlidePrompt::default());
            }
            if ui.button("File In").clicked() {
                commands.insert_resource(PersistConfirmationDialog(PersistenceEvent::FileIn));
            }
            if ui.button("File Out").clicked() {
                commands.insert_resource(PersistConfirmationDialog(PersistenceEvent::FileOut));
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

struct SlideEditor {
    target: Entity,
    unsaved: Slide,
    info: String,
}

impl SlideEditor {
    fn new_for(target: Entity, slide: &Slide) -> Self {
        Self {
            target,
            unsaved: slide.clone(),
            info: String::new(),
        }
    }
    fn render(
        egui_context: ResMut<EguiContext>,
        mut editors: Query<(Entity, &mut Self)>,
        slides: Query<(Entity, &Slide)>,
        mut commands: Commands,
    ) {
        let valid_slide_names: Vec<_> = slides.iter().map(|(_, s)| s.name.clone()).collect();

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
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut e.unsaved.name);
                    });
                    ui.label("Description:");
                    ui.text_edit_multiline(&mut e.unsaved.description);
                    ui.label("Actions:");
                    ScrollArea::auto_sized().show(ui, |ui| {
                        let mut to_remove = vec![];
                        for (i, a) in e.unsaved.actions.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut a.text);
                                egui::ComboBox::from_id_source((eid, i))
                                    .selected_text(&a.target_slide)
                                    .show_ui(ui, |ui| {
                                        for sn in valid_slide_names.iter() {
                                            ui.selectable_value(&mut a.target_slide, sn.clone(), sn);
                                        }
                                    });
                                if ui.small_button("x").clicked() {
                                    to_remove.push(a.clone());
                                }
                            });
                        }
                        e.unsaved.actions.retain(|a| !to_remove.contains(a));
                        if ui.small_button("Add action").clicked() {
                            e.unsaved.actions.push(Action::default());
                        }
                    });
                    ui.separator();
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
struct AddSlidePrompt {
    name: String,
    info: String,
}

impl AddSlidePrompt {
    fn render(
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
