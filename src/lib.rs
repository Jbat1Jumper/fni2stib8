use bevy::{
    app::AppExit,
    diagnostic::Diagnostics,
    input::keyboard::KeyboardInput,
    log::{Level, LogSettings},
};

#[macro_use]
use bevy::prelude::*;
//use bevy::render::camera::OrthographicProjection;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use model::EditorsOpen;
use wasm_bindgen::prelude::*;

mod editors;
mod images;
mod model;
mod persistence;
mod player;

use crate::persistence::PersistenceEvent;

#[wasm_bindgen]
pub fn run() {
    let mut app = App::build();
    app.insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
        .add_plugins(DefaultPlugins);

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.insert_resource(LogSettings {
        level: Level::DEBUG,
        ..Default::default()
    })
    .insert_resource(EditorsOpen(false))
    .insert_resource(EguiSettings { scale_factor: 1.0 })
    .add_plugin(EguiPlugin)
    .add_plugin(model::ModelPlugin)
    .add_plugin(images::ImagesPlugin)
    .add_plugin(persistence::PersistencePlugin::<model::Slide>::new())
    .add_plugin(editors::EditorsPlugin)
    .add_plugin(player::PlayerPlugin)
    .add_system(PersistConfirmationDialog::render.system())
    .add_startup_system(on_startup.system())
    .add_system(debug.system())
    .run();
}

fn on_startup(
    mut commands: Commands,
    mut persistence: EventWriter<PersistenceEvent<model::Slide>>,
    mut persistence_bg: EventWriter<PersistenceEvent<images::Background>>,
) {
    info!("Started!");
    persistence.send(PersistenceEvent::FileIn);
    persistence_bg.send(PersistenceEvent::FileIn);
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn debug(
    egui_context: ResMut<EguiContext>,
    mut commands: Commands,
    mut app_exit: EventWriter<AppExit>,
    mut buttons: EventReader<KeyboardInput>,
    mut editors_open: ResMut<EditorsOpen>,
) {
    // std::thread::sleep_ms(50);
    if editors_open.0 {
        egui::Window::new("Main menu").show(egui_context.ctx(), |ui| {
            if ui.button("File In").clicked() {
                commands.insert_resource(PersistConfirmationDialog(PersistenceEvent::FileIn));
            }
            if ui.button("File Out").clicked() {
                commands.insert_resource(PersistConfirmationDialog(PersistenceEvent::FileOut));
            }
            if ui.button("Quit").clicked() {
                app_exit.send(AppExit);
            }
        });
    }

    for ev in buttons.iter() {
        if ev.key_code == Some(KeyCode::E) && ev.state.is_pressed() {
            editors_open.0 = !editors_open.0;
        }
    }
}

struct PersistConfirmationDialog(PersistenceEvent<()>);

impl PersistConfirmationDialog {
    fn render(
        egui_context: ResMut<EguiContext>,
        mut commands: Commands,
        dialog: Option<Res<Self>>,
        mut slide_persistence: EventWriter<PersistenceEvent<model::Slide>>,
        mut bg_persistence: EventWriter<PersistenceEvent<images::Background>>,
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
                    let bg_ev = match dialog.0 {
                        PersistenceEvent::FileIn => PersistenceEvent::FileIn,
                        PersistenceEvent::FileOut => PersistenceEvent::FileOut,
                        PersistenceEvent::_Phantom(_) => unreachable!(),
                    };
                    bg_persistence.send(bg_ev);
                    let slide_ev = match dialog.0 {
                        PersistenceEvent::FileIn => PersistenceEvent::FileIn,
                        PersistenceEvent::FileOut => PersistenceEvent::FileOut,
                        PersistenceEvent::_Phantom(_) => unreachable!(),
                    };
                    slide_persistence.send(slide_ev);
                    commands.remove_resource::<Self>();
                }
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<Self>();
                }
            });
        });
    }
}
