use crate::{images::{Background, BackgroundData, convert_background_to_ascii}, model::*};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, ScrollArea, TextEdit},
    EguiContext,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .insert_resource(Player::new())
            .add_startup_system(Player::startup.system())
            .add_system(Player::render_controls.system())
            .add_system(Player::render.system())
            .add_system(Player::handle_renames.system());
    }
}

struct DisplayText;

#[derive(Debug, Default)]
struct Player {
    current_slide: String,
    current_rendered_text: String,
    render_timer: Timer,
}

impl Player {
    fn new() -> Self {
        Self {
            current_slide: "Living".into(),
            current_rendered_text: ".".into(),
            render_timer: Timer::from_seconds(1., true),
        }
    }
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
    fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
        info!("Player starting up");
        commands.spawn_bundle(Text2dBundle {
            text: Text::with_section(
                "Loading",
                TextStyle {
                    font: asset_server.load("fonts/BPtypewrite.otf"),
                    font_size: 10.0,
                    color: Color::WHITE,
                    //..Default::default()
                },
                TextAlignment {
                    vertical: VerticalAlign::Center,
                    horizontal: HorizontalAlign::Center,
                },
            ),
            ..Default::default()
        }).insert(DisplayText);
    }

    fn render(
        mut player: ResMut<Self>,
        slides: Query<&Slide>,
        time: Res<Time>,
        backgrounds: Query<(&Background, &BackgroundData)>,
        mut text: Query<&mut Text, With<DisplayText>>,
        mut commands: Commands,
    ) {
        if !player.render_timer.tick(time.delta()).just_finished() {
            return;
        }
        let rendered_text = match slides
                .iter()
                .find(|slide| slide.name == player.current_slide) {
                    None => "-.- Slide not found -'-".into(),
                    Some(slide) => {
                        match backgrounds.iter().find(|(bg, _)| bg.name() == slide.background) {
                            None => "-'- No background -.-".into(),
                            Some((bg, bgd)) => convert_background_to_ascii(bg, bgd),
                        }

                    }
                };
        if player.current_rendered_text != rendered_text {
            info!("Changed rendered text");
            for mut t in text.iter_mut() {
                info!("Changing text in a text component");
                t.sections.first_mut().unwrap().value = rendered_text.clone();
            }
            player.current_rendered_text = rendered_text;
        }
    }

    fn render_controls(
        mut player: ResMut<Self>,
        slides: Query<&Slide>,
        egui_context: ResMut<EguiContext>,
        mut text: Query<(&mut Text, &mut Transform), With<DisplayText>>,
        mut commands: Commands,
    ) {
        let valid_slide_names: Vec<_> = slides.iter().map(|s| s.name.clone()).collect();
        egui::Window::new("Player Controls").show(egui_context.ctx(), |ui| {
            egui::ComboBox::from_label("Current slide")
                .selected_text(&player.current_slide)
                .show_ui(ui, |ui| {
                    for sn in valid_slide_names.iter() {
                        ui.selectable_value(&mut player.current_slide, sn.clone(), sn);
                    }
                });
            ui.separator();
            for (mut text, mut transform) in text.iter_mut() {
                ui.label("Text position");
                ui.add(egui::DragValue::new(&mut transform.translation.x).speed(1.));
                ui.add(egui::DragValue::new(&mut transform.translation.y).speed(1.));

                ui.label("Font size");
                ui.add(egui::DragValue::new(&mut text.sections.first_mut().unwrap().style.font_size).speed(1.).clamp_range(6..=40));
            }
            ui.separator();

            let s = slides
                .iter()
                .find(|slide| slide.name == player.current_slide);
            if s.is_none() {
                ui.colored_label(egui::Color32::RED, "The current slide does not exist");
                return;
            }
            let scene = s.unwrap();
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
