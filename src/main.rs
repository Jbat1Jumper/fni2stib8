use bevy::{
    app::AppExit,
    diagnostic::Diagnostics,
    log::{Level, LogSettings},
};
#[macro_use]
use bevy::prelude::*;
//use bevy::render::camera::OrthographicProjection;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};

mod editors;
mod model;
mod persistence;
mod player;

use crate::persistence::PersistenceEvent;

pub fn main() {
    App::build()
        .insert_resource(LogSettings {
            level: Level::DEBUG,
            ..Default::default()
        })
        .insert_resource(EguiSettings { scale_factor: 1.2 })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(model::ModelPlugin)
        .add_plugin(persistence::PersistencePlugin)
        .add_plugin(editors::EditorsPlugin)
        .add_plugin(player::PlayerPlugin)
        .add_startup_system(on_startup.system())
        .add_system(debug.system())
        .run();
}

fn on_startup(mut _commands: Commands, mut persistence: EventWriter<PersistenceEvent>) {
    info!("Started!");
    persistence.send(PersistenceEvent::FileIn);
}

fn debug(
    egui_context: ResMut<EguiContext>,
    mut commands: Commands,
    mut app_exit: EventWriter<AppExit>,
) {
    egui::Window::new("Main menu").show(egui_context.ctx(), |ui| {
        if ui.button("Quit").clicked() {
            app_exit.send(AppExit);
        }

        if ui.button("Start").clicked() {
            commands.insert_resource(player::StartPrompt("".into()));
        }

    });
}
