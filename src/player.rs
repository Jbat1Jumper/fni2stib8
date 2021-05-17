use crate::{
    images::{convert_background_to_ascii, Background, BackgroundData},
    model::*,
};
use bevy::{input::{ElementState, mouse::MouseButtonInput}, prelude::*, render::camera::Camera};
use bevy_egui::{
    egui::{self, ScrollArea, TextEdit},
    EguiContext,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        builder
            .insert_resource(Player::new())
            .insert_resource(PlayerState::FadeInBg(Timer::from_seconds(
                BG_FADE_IN, false,
            )))
            .add_startup_system(Player::startup.system())
            .add_system(Player::render_controls.system())
            .add_system(Player::render.system())
            .add_system(Player::handle_mouse.system())
            .add_system(Player::update_state.system())
            .add_system(Player::handle_renames.system());
    }
}

struct DisplayBackground;
struct DisplayDescription;
struct DisplayActions;

#[derive(Debug)]
struct Player {
    current_slide: String,
    next_slide: String,
    bg_opacity: f32,
    percentage_of_text_shown: f32,
    amount_of_actions_shown: f32,
    render_timer: Timer,
    hovering_action: Option<usize>,
    dashes: usize,
    render: bool,
    pauses: f32,
    action_pause: f32,
}

#[derive(Debug, Clone)]
enum PlayerState {
    FadeInBg(Timer),
    PauseBetweenBgAndText(Timer),
    FadeInText(Timer),
    PauseBetweenTextAndActions(Timer),
    FadeInActions(Timer),
    WaitingForInput(Timer),
    GotInput,
    FadeOutTextAndActions(Timer),
    FadeOutBg(Timer),
}

impl Player {
    fn update_state(
        mut player: ResMut<Self>,
        mut state: ResMut<PlayerState>,
        time: Res<Time>,
        slides: Query<&Slide>,
    ) {
        let slide = slides
            .iter()
            .find(|slide| slide.name == player.current_slide);

        if slide.is_none() {
            return;
        }
        let slide = slide.unwrap();
        let text_fade_in_duration =
            (slide.description.len() as f32 / MEAN_WORD_LENGTH) / MEAN_READING_SPEED_WPS;

        use PlayerState::*;
        match *state {
            FadeInBg(ref mut timer) => {
                player.bg_opacity = timer.percent();
                if timer.tick(time.delta()).just_finished() {
                    info!("FadeInBg finished");
                    player.bg_opacity = 1.0;
                    *state = PauseBetweenBgAndText(Timer::from_seconds(player.pauses, false))
                }
            }
            PauseBetweenBgAndText(ref mut timer) => {
                if timer.tick(time.delta()).just_finished() {
                    info!("PauseBetweenBgAndText finished");
                    *state = FadeInText(Timer::from_seconds(text_fade_in_duration, false))
                }
            }
            FadeInText(ref mut timer) => {
                player.percentage_of_text_shown = timer.percent();
                if timer.tick(time.delta()).just_finished() {
                    info!("FadeInText finished");
                    player.percentage_of_text_shown = 1.0;
                    *state = PauseBetweenTextAndActions(Timer::from_seconds(player.pauses, false))
                }
            }
            PauseBetweenTextAndActions(ref mut timer) => {
                if timer.tick(time.delta()).just_finished() {
                    info!("PauseBetweenTextAndActions finished");
                    *state = FadeInActions(Timer::from_seconds(
                        slide.actions.len() as f32 * player.action_pause,
                        false,
                    ))
                }
            }
            FadeInActions(ref mut timer) => {
                player.amount_of_actions_shown = timer.percent();
                if timer.tick(time.delta()).just_finished() {
                    info!("FadeInActions finished");
                    player.amount_of_actions_shown = 1.0;
                    *state = WaitingForInput(Timer::from_seconds(1.0, true));
                }
            }
            WaitingForInput(ref mut timer) => {
                player.dashes = (timer.percent() * 4.0) as usize;
                if timer.tick(time.delta()).just_finished() {}
            }
            GotInput => *state = FadeOutTextAndActions(Timer::from_seconds(0.5, false)),
            FadeOutTextAndActions(ref mut timer) => {
                player.percentage_of_text_shown = 1.0 - timer.percent();
                player.amount_of_actions_shown = 1.0 - timer.percent();
                if timer.tick(time.delta()).just_finished() {
                    info!("FadeOutTextAndActions finished");
                    player.percentage_of_text_shown = 0.0;
                    player.amount_of_actions_shown = 0.0;
                    *state = FadeOutBg(Timer::from_seconds(BG_FADE_OUT, false))
                }
            }
            FadeOutBg(ref mut timer) => {
                player.bg_opacity = 1.0 - timer.percent();
                if timer.tick(time.delta()).just_finished() {
                    info!("FadeOutBg finished");
                    player.bg_opacity = 0.0;
                    player.current_slide = player.next_slide.clone();
                    *state = FadeInBg(Timer::from_seconds(BG_FADE_IN, false))
                }
            }
            _ => {}
        }
    }
}

const BG_FADE_IN: f32 = 3.0 / 2.0;
const BG_FADE_OUT: f32 = 2.0 / 2.0;
const MEAN_WORD_LENGTH: f32 = 4.7;
const MEAN_READING_SPEED_WPS: f32 = 3.6 * 5.0;

impl Player {
    fn new() -> Self {
        Self {
            current_slide: "Living".into(),
            next_slide: "Living".into(),
            render_timer: Timer::from_seconds(0.1, true),
            render: true,
            pauses: 0.1, // 1.0
            dashes: 0,
            percentage_of_text_shown: 0.0,
            amount_of_actions_shown: 0.0,
            hovering_action: None,
            bg_opacity: 0.0,
            action_pause: 0.1,  // 1.0
        }
    }
}

fn window_to_world(
    window: &Window,
    camera: &Transform,
    position: &Vec2,
) -> Vec3 {
    let center = camera.translation.truncate();
    let half_width = (window.width() / 2.0) * camera.scale.x;
    let half_height = (window.height() / 2.0) * camera.scale.y;
    let left = center.x - half_width;
    let bottom = center.y - half_height;
    Vec3::new(
        left + position.x * camera.scale.x,
        bottom + position.y * camera.scale.y,
        0.0,  // I'm working in 2D
    )
}

impl Player {
    fn handle_mouse(
        mut cursor_moved: EventReader<CursorMoved>,
        mut mouse_button: EventReader<MouseButtonInput>,
        mut player: ResMut<Self>,
        mut player_state: ResMut<PlayerState>,
        transforms: QuerySet<(
            Query<&Transform, With<DisplayActions>>,
            Query<&Transform, With<Camera>>,
        )>,
        windows: Res<Windows>,
        time: Res<Time>,
        slides: Query<&Slide>,
    ) {
        let slide = slides
            .iter()
            .find(|slide| slide.name == player.current_slide);
        if slide.is_none() {
            return;
        }
        let slide = slide.unwrap();
        let offset_between_actions = 30.0;
        let mut action_origin = 0.0;
        let min_distance = 30;
        for at in transforms.q0().iter() {
            action_origin = at.translation.y;
        }
        let camera = transforms.q1().iter().find(|_| true).unwrap();
        for CursorMoved { position, id } in cursor_moved.iter() {
            let window = windows.get(*id).unwrap();
            let position = window_to_world(window, camera, position);
            info!("CursorMoved {:?}", position);
            let mut distances = vec![];
            for (i, _a) in slide.actions.iter().enumerate() {
                let d =
                    (action_origin - i as f32 * offset_between_actions - position.y).abs() as i32;
                if d < min_distance {
                    distances.push((i, d));
                }
            }
            player.hovering_action = distances
                .into_iter()
                .min_by_key(|(_, distance_to_mouse)| *distance_to_mouse)
                .map(|(i, _)| i);
        }
        for MouseButtonInput { button, state } in mouse_button.iter() {
            if *button == MouseButton::Left && state.is_pressed() {

                if let Some(i) = player.hovering_action.clone() {
                    let a = slide.actions.get(i).unwrap();
                    player.next_slide = a.target_slide.clone();
                    *player_state = PlayerState::GotInput;
                }
            }
        }
    }

    fn handle_renames(mut player: ResMut<Self>, mut slide_events: EventReader<CrudEvent<Slide>>) {
        for ev in slide_events.iter() {
            match ev {
                CrudEvent::Renamed(old_name, new_name) => {
                    if player.current_slide == *old_name {
                        player.current_slide = new_name.clone();
                    }
                }
                _ => {}
            }
        }
    }
    fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
        info!("Player starting up");
        commands
            .spawn_bundle(Text2dBundle {
                text: Text::with_section(
                    "Loading",
                    TextStyle {
                        font: asset_server.load("fonts/BPtypewrite.otf"),
                        font_size: 12.0,
                        color: Color::WHITE,
                        //..Default::default()
                    },
                    TextAlignment {
                        vertical: VerticalAlign::Bottom,
                        horizontal: HorizontalAlign::Center,
                    },
                ),
                transform: Transform {
                    translation: Vec3::new(000.0, 380.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(DisplayBackground);
        commands
            .spawn_bundle(Text2dBundle {
                text: Text::with_section(
                    "Description",
                    TextStyle {
                        font: asset_server.load("fonts/BPtypewrite.otf"),
                        font_size: 12.0,
                        color: Color::WHITE,
                        //..Default::default()
                    },
                    TextAlignment {
                        vertical: VerticalAlign::Bottom,
                        horizontal: HorizontalAlign::Center,
                    },
                ),
                transform: Transform {
                    translation: Vec3::new(000.0, -30.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(DisplayDescription);
        commands
            .spawn_bundle(Text2dBundle {
                text: Text::with_section(
                    "Actions",
                    TextStyle {
                        font: asset_server.load("fonts/BPtypewrite.otf"),
                        font_size: 12.0,
                        color: Color::WHITE,
                        //..Default::default()
                    },
                    TextAlignment {
                        vertical: VerticalAlign::Bottom,
                        horizontal: HorizontalAlign::Center,
                    },
                ),
                transform: Transform {
                    translation: Vec3::new(000.0, -100.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(DisplayActions);
    }

    fn render(
        mut player: ResMut<Self>,
        slides: Query<&Slide>,
        time: Res<Time>,
        backgrounds: Query<(&Background, &BackgroundData)>,
        mut texts: QuerySet<(
            Query<&mut Text, With<DisplayBackground>>,
            Query<&mut Text, With<DisplayDescription>>,
            Query<&mut Text, With<DisplayActions>>,
        )>,
        mut commands: Commands,
    ) {
        if !player.render_timer.tick(time.delta()).just_finished() || !player.render {
            return;
        }
        match slides
            .iter()
            .find(|slide| slide.name == player.current_slide)
        {
            None => {
                warn!("slide not found");
            }
            Some(slide) => {
                match backgrounds
                    .iter()
                    .find(|(bg, _)| bg.name() == slide.background)
                {
                    None => warn!("background not found"),
                    Some((bg, bgd)) => {
                        let rendered_text = convert_background_to_ascii(bg, bgd, player.bg_opacity);
                        for mut t in texts.q0_mut().iter_mut() {
                            if t.sections.first().unwrap().value != rendered_text {
                                info!("Changing text in a bg_text component");
                                t.sections.first_mut().unwrap().value = rendered_text.clone();
                            }
                        }

                        let mut description_text = slide.description.clone();
                        description_text.truncate(
                            (description_text.len() as f32 * player.percentage_of_text_shown)
                                as usize,
                        );
                        for mut t in texts.q1_mut().iter_mut() {
                            if t.sections.first().unwrap().value != description_text {
                                info!("Changing text in a desc_text component");
                                t.sections.first_mut().unwrap().value = description_text.clone();
                            }
                        }

                        let mut actions_text = String::new();
                        let n_actions =
                            (slide.actions.len() as f32 * player.amount_of_actions_shown) as usize;
                        for (i, a) in slide.actions.iter().enumerate().take(n_actions) {
                            if player.hovering_action == Some(i) {
                                actions_text += &"-".repeat(player.dashes);
                                actions_text += "> ";
                                actions_text += &" ".repeat(5 - player.dashes);
                            }
                            actions_text += &a.text;
                            if player.hovering_action == Some(i) {
                                actions_text += &" ".repeat(5 - player.dashes);
                                actions_text += " <";
                                actions_text += &"-".repeat(player.dashes);
                            }
                            actions_text += "\n\n";
                        }
                        for mut t in texts.q2_mut().iter_mut() {
                            if t.sections.first().unwrap().value != actions_text {
                                info!("Changing text in an action text component");
                                t.sections.first_mut().unwrap().value = actions_text.clone();
                            }
                        }
                    }
                };
            }
        };
    }

    fn render_controls(
        mut player: ResMut<Self>,
        mut player_state: ResMut<PlayerState>,
        slides: Query<&Slide>,
        egui_context: ResMut<EguiContext>,
        mut texts: Query<
            (&mut Text, &mut Transform),
            Or<(
                With<DisplayBackground>,
                With<DisplayActions>,
                With<DisplayDescription>,
            )>,
        >,
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
            //ui.checkbox(&mut player.render, "Render on");
            //ui.label(format!("{:#?}", *player_state));
            //ui.separator();
            // ui.separator();
            // for (mut text, mut transform) in texts.iter_mut() {
            //     ui.label("Text position");
            //     ui.add(egui::DragValue::new(&mut transform.translation.x).speed(1.));
            //     ui.add(egui::DragValue::new(&mut transform.translation.y).speed(1.));

            //     ui.label("Font size");
            //     ui.add(
            //         egui::DragValue::new(&mut text.sections.first_mut().unwrap().style.font_size)
            //             .speed(1.)
            //             .clamp_range(6..=40),
            //     );
            //     ui.separator();
            // }
            // ui.separator();

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
            for (i, a) in scene.actions.iter().enumerate() {
                let res = ui.button(&a.text);
                if res.clicked() {
                    player.next_slide = a.target_slide.clone();
                    *player_state = PlayerState::GotInput;
                }
                if res.hovered() {
                    player.hovering_action = Some(i);
                }
            }
        });
    }
}
