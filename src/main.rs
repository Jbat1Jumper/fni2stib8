use bevy::{
    app::AppExit,
    diagnostic::Diagnostics,
    log::{Level, LogSettings},
};
#[macro_use]
use bevy::prelude::*;
//use bevy::render::camera::OrthographicProjection;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};

mod model;
mod persistence;
mod editors;
use model::*;
use editors::*;
use persistence::PersistencePlugin;

pub fn main() {
    App::build()
        .insert_resource(LogSettings {
            level: Level::DEBUG,
            ..Default::default()
        })
        .insert_resource(EguiSettings { scale_factor: 2.0 })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(PersistencePlugin)
        .add_startup_system(on_startup.system())
        .add_system(debug.system())
        .add_system(AddSlidePrompt::render.system())
        .add_system(SlideEditor::render.system())
        .add_system(slide_list.system())
        .run();
}

fn on_startup(mut _commands: Commands) {
    info!("Started!");
}

fn debug(
    egui_context: ResMut<EguiContext>,
    mut _commands: Commands,
    mut app_exit: EventWriter<AppExit>,
) {
    egui::Window::new("Main menu").show(egui_context.ctx(), |ui| {
        if ui.button("Quit").clicked() {
            app_exit.send(AppExit);
        }
    });
}

