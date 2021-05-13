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
            .insert_resource(Player::default())
            .add_system(Player::render.system())
            .add_system(Player::handle_renames.system());
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
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
        player: Option<ResMut<Self>>,
        slides: Query<(Entity, &Slide)>,
        egui_context: ResMut<EguiContext>,
        mut commands: Commands,
    ) {
        if player.is_none() {
            return;
        }
        let mut player = player.unwrap();
        let valid_slide_names: Vec<_> = slides.iter().map(|(_, s)| s.name.clone()).collect();
        egui::Window::new("Player Controls").show(egui_context.ctx(), |ui| {
            egui::ComboBox::from_label("Current slide")
                .selected_text(&player.current_slide)
                .show_ui(ui, |ui| {
                    for sn in valid_slide_names.iter() {
                        ui.selectable_value(&mut player.current_slide, sn.clone(), sn);
                    }
                });
            ui.separator();

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
    }
}
