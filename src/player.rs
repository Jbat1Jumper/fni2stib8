use crate::model::*;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, ScrollArea, TextEdit},
    EguiContext,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .add_system(Player::render.system())
            .add_system(Player::handle_renames.system())
            .add_system(StartPrompt::render.system());
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct StartPrompt(pub String);

impl StartPrompt {
    fn render(
        slides: Query<(Entity, &Slide)>,
        egui_context: ResMut<EguiContext>,
        mut commands: Commands,
        prompt: Option<ResMut<Self>>,
    ) {
        if prompt.is_none() {
            return;
        }
        let mut prompt = prompt.unwrap();
        let valid_slide_names: Vec<_> = slides.iter().map(|(_, s)| s.name.clone()).collect();
        egui::Window::new("Start game").show(egui_context.ctx(), |ui| {
            egui::ComboBox::from_label("From slide")
                .selected_text(&prompt.0)
                .show_ui(ui, |ui| {
                    for sn in valid_slide_names.iter() {
                        ui.selectable_value(&mut prompt.0, sn.clone(), sn);
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    commands.spawn().insert(Player {
                        current_slide: prompt.0.clone(),
                    });
                    commands.remove_resource::<Self>();
                }
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<Self>();
                }
            });
        });
    }
}

struct Player {
    current_slide: String,
}

impl Player {
    fn handle_renames(
        mut players: Query<&mut Self>,
        mut slide_events: EventReader<CrudEvent<Slide>>,
    ) {
        for ev in slide_events.iter() {
            match ev {
                CrudEvent::Renamed(old_name, new_name) => {
                    for mut e in players.iter_mut() {
                        if e.current_slide == *old_name {
                            e.current_slide = new_name.clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    fn render(
        mut players: Query<(Entity, &mut Player)>,
        slides: Query<(Entity, &Slide)>,
        egui_context: ResMut<EguiContext>,
        mut commands: Commands,
    ) {
        for (player_entity, mut player) in players.iter_mut() {
            let mut open = true;
            egui::Window::new(format!("Player {}", player.current_slide))
                .open(&mut open)
                .id(egui::Id::new(player_entity))
                .show(egui_context.ctx(), |ui| {
                    let s = slides
                        .iter()
                        .find(|(_, slide)| slide.name == player.current_slide);
                    if s.is_none() {
                        ui.colored_label(egui::Color32::RED, "The current slide does not exist");
                        return;
                    }
                    let (_scene_entity, scene) = s.unwrap();

                    ui.add(
                        TextEdit::multiline(&mut scene.description.clone())
                            .text_style(egui::TextStyle::Monospace)
                            .enabled(false),
                    );
                    ui.separator();
                    for a in scene.actions.iter() {
                        if ui.button(&a.text).clicked() {
                            player.current_slide = a.target_slide.clone();
                        }
                    }
                });
            if !open {
                commands.entity(player_entity).despawn();
            }
        }
    }
}
