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
            .add_system(slide_list.system())
            .add_system(SlideEditor::render.system())
            .add_system(SlideEditor::handle_renames.system())
            .add_system(AddSlidePrompt::render.system())
            .add_system(RenameSlideDialog::render.system())
            .add_system(DeleteSlideDialog::render.system())
            .add_system(PersistConfirmationDialog::render.system());
    }
}
struct DeleteSlideDialog(String);
impl DeleteSlideDialog {
    fn render(
        egui_context: ResMut<EguiContext>,
        dialog: Option<ResMut<Self>>,
        mut slide_events: EventWriter<CrudEvent<Slide>>,
        mut commands: Commands,
        slides: Query<&Slide>,
    ) {
        if dialog.is_none() {
            return;
        }
        let mut dialog = dialog.unwrap();

        let slides_with_references: Vec<_> = slides
            .iter()
            .filter(|s| s.actions.iter().any(|a| a.target_slide == dialog.0))
            .map(|s| s.name.clone())
            .collect();

        egui::Window::new("Delete slide").show(egui_context.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Goging to delete \"{}\"", dialog.0));
            });
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<Self>();
                }
                if slides_with_references.is_empty() {
                    if ui.button("Dlete").clicked() {
                        slide_events.send(CrudEvent::Deleted(dialog.0.clone()));
                        commands.remove_resource::<Self>();
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "Cant delete, has references from:");
                    for r in slides_with_references {
                        ui.label(r);
                    }
                }
            });
        });
    }
}

struct RenameSlideDialog(String, String);

impl RenameSlideDialog {
    fn render(
        egui_context: ResMut<EguiContext>,
        dialog: Option<ResMut<Self>>,
        mut slide_events: EventWriter<CrudEvent<Slide>>,
        mut commands: Commands,
        slides: Query<&Slide>,
    ) {
        if dialog.is_none() {
            return;
        }
        let mut dialog = dialog.unwrap();

        egui::Window::new("Rename slide").show(egui_context.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("\"{}\" will now be called", dialog.0));
                ui.text_edit_singleline(&mut dialog.1);
            });

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<Self>();
                }

                if dialog.1.is_empty() {
                    ui.colored_label(egui::Color32::RED, "Name cant be empty");
                } else if dialog.0 != dialog.1 && slides.iter().any(|s| s.name == dialog.1) {
                    ui.colored_label(egui::Color32::RED, "Name already taken");
                } else {
                    if ui.button("Confirm rename").clicked() {
                        slide_events.send(CrudEvent::Renamed(dialog.0.clone(), dialog.1.clone()));
                        commands.remove_resource::<Self>();
                    }
                }
            });
        });
    }
}

struct PersistConfirmationDialog(PersistenceEvent<Slide>);

impl PersistConfirmationDialog {
    fn render(
        egui_context: ResMut<EguiContext>,
        mut commands: Commands,
        dialog: Option<Res<Self>>,
        mut fs_event: EventWriter<PersistenceEvent<Slide>>,
    ) {
        if dialog.is_none() {
            return;
        }
        let dialog = dialog.unwrap();

        egui::Window::new("Please confirm").show(egui_context.ctx(), |ui| {
            ui.label(match dialog.0 {
                PersistenceEvent::FileIn => "Doing a File In will erase all your unsaved changes.",
                PersistenceEvent::FileOut => "Doing a File Out will override the file",
                PersistenceEvent::_Phantom(_) => unreachable!(),
            });
            ui.horizontal(|ui| {
                if ui.button("Proceed").clicked() {
                    fs_event.send(dialog.0.clone());
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
    slides: Query<&Slide>,
    mut commands: Commands,
    mut slide_events: EventWriter<CrudEvent<Slide>>,
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

        for s in slides.iter() {
            ui.horizontal(|ui| {
                ui.label(format!("{}", s.name,));
                if ui.small_button("edit").clicked() {
                    commands.spawn().insert(SlideEditor::new_for(&s.name));
                }
                if ui.small_button("remove").clicked() {
                    commands.insert_resource(DeleteSlideDialog(s.name.clone()));
                }
                if ui.small_button("rename").clicked() {
                    commands.insert_resource(RenameSlideDialog(s.name.clone(), s.name.clone()));
                }
            });
        }
    });
}

struct SlideEditor {
    target: String,
    ttl: usize,
}

impl SlideEditor {
    fn new_for(slide_name: &str) -> Self {
        Self {
            target: slide_name.into(),
            ttl: 3,
        }
    }
    fn handle_renames(mut editors: Query<&mut Self>, mut slide_events: EventReader<CrudEvent<Slide>>) {
        for ev in slide_events.iter() {
            match ev {
                CrudEvent::Renamed(old_name, new_name) => {
                    for mut e in editors.iter_mut() {
                        if e.target == *old_name {
                            e.target = new_name.clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    fn render(
        egui_context: ResMut<EguiContext>,
        mut editors: Query<(Entity, &mut Self)>,
        slides: Query<&Slide>,
        mut slide_events: EventWriter<CrudEvent<Slide>>,
        mut commands: Commands,
    ) {
        let valid_slide_names: Vec<_> = slides.iter().map(|s| s.name.clone()).collect();

        for (eid, mut e) in editors.iter_mut() {
            let saved = match slides.iter().filter(|s| s.name == e.target).next() {
                None => {
                    if e.ttl > 0 {
                        warn!("{} not found, closing editor in {}", e.target, e.ttl);
                        e.ttl -= 1;
                    } else {
                        warn!("{} not found, editor closed", e.target);
                        commands.entity(eid).despawn();
                    }
                    continue;
                }
                Some(s) => s,
            };
            let mut unsaved = saved.clone();
            let mut open = true;
            egui::Window::new(format!("Edit: {}", saved.name))
                .id(egui::Id::new(eid))
                .open(&mut open)
                .show(egui_context.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.label(&unsaved.name);
                        if ui.small_button("Rename").clicked() {
                            commands.insert_resource(RenameSlideDialog(
                                unsaved.name.clone(),
                                unsaved.name.clone(),
                            ));
                        }
                    });
                    ui.label("Description:");
                    ui.text_edit_multiline(&mut unsaved.description);
                    ui.label("Actions:");
                    ScrollArea::auto_sized().show(ui, |ui| {
                        let mut to_remove = vec![];
                        for (i, a) in unsaved.actions.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut a.text);
                                egui::ComboBox::from_id_source((eid, i))
                                    .selected_text(&a.target_slide)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut a.target_slide,
                                            "*NEW*".into(),
                                            "*NEW*",
                                        );
                                        for sn in valid_slide_names.iter() {
                                            ui.selectable_value(
                                                &mut a.target_slide,
                                                sn.clone(),
                                                sn,
                                            );
                                        }
                                    });

                                if a.target_slide == "*NEW*" {
                                    let name = (0..10000)
                                        .map(|n| format!("slide{}", n))
                                        .filter(|name| !valid_slide_names.contains(name))
                                        .next()
                                        .expect("Abusrd amount of badly named slides");
                                    commands.spawn().insert(Slide::new(name.clone()));
                                    commands.spawn().insert(SlideEditor::new_for(&name));
                                    a.target_slide = name;
                                }

                                if ui.small_button("->").clicked() {
                                    commands
                                        .spawn()
                                        .insert(SlideEditor::new_for(&a.target_slide));
                                }
                                if ui.small_button("x").clicked() {
                                    to_remove.push(a.clone());
                                }
                            });
                        }
                        unsaved.actions.retain(|a| !to_remove.contains(a));
                        if ui.small_button("Add action").clicked() {
                            unsaved.actions.push(Action::default());
                        }
                    });

                    let slides_with_references: Vec<_> = slides
                        .iter()
                        .filter(|s| s.actions.iter().any(|a| a.target_slide == e.target))
                        .map(|s| s.name.clone())
                        .collect();

                    ui.separator();
                    ui.label("Slides referencing this one:");
                    ui.horizontal(|ui| {
                        for rs in slides_with_references {
                            if ui.small_button(&rs).clicked() {
                                commands.spawn().insert(SlideEditor::new_for(&rs));
                            }
                        }
                    });
                });
            if !open {
                commands.entity(eid).despawn();
            }
            if unsaved != *saved {
                slide_events.send(CrudEvent::Updated(unsaved.clone()));
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
        mut slide_events: EventWriter<CrudEvent<Slide>>,
        slides: Query<(Entity, &Slide)>,
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
                        match validate_name(&prompt.name, None, &slides) {
                            Err(e) => {
                                prompt.info = e.into();
                                return;
                            }
                            _ => {}
                        }

                        let slide = Slide::new(prompt.name.clone());

                        slide_events.send(CrudEvent::Created(slide.clone()));
                        commands.spawn().insert(SlideEditor::new_for(&slide.name));
                        commands.remove_resource::<AddSlidePrompt>();
                    }
                });
                ui.separator();
                ui.label(&prompt.info);
            });
        }
    }
}

fn validate_name(
    name: &str,
    entity_claiming_name: Option<Entity>,
    query: &Query<(Entity, &Slide)>,
) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Name can not be empty");
    }

    match get_slide_entity_by_name(name, query) {
        None => Ok(()),
        Some(e) if Some(e) == entity_claiming_name => Ok(()),
        Some(_other_e) => Err("There is another slide with that name"),
    }
}

fn get_slide_entity_by_name(name: &str, query: &Query<(Entity, &Slide)>) -> Option<Entity> {
    query
        .iter()
        .filter(|(_, s)| s.name == name)
        .map(|(e, _)| e)
        .next()
}
