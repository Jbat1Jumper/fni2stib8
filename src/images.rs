use serde::*;
use std::{
    io::Read,
    path::Path,
    thread::{spawn, JoinHandle},
};

use bevy::{
    prelude::*,
    render::texture::{Extent3d, ImageType, TextureDimension, TextureFormat},
};
use bevy_egui::{egui, EguiContext};
use image::{GenericImageView, RgbaImage};

const WIDTH: u32 = 128;
const HEIGHT: u32 = WIDTH / 4;

fn load_from_bytes(bytes: &[u8]) -> (RgbaImage, Texture) {
    let reader = image::io::Reader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .expect("Cursor never fails");
    let image = reader
        .decode()
        .expect("Failed to decode image")
        .resize_to_fill(WIDTH, HEIGHT * 2, image::imageops::FilterType::Nearest)
        .resize_exact(WIDTH, HEIGHT, image::imageops::FilterType::Nearest)
        .to_rgba8();
    let data: Vec<u8> = image.pixels().map(|p| &p.0).flatten().cloned().collect();
    let size = image.dimensions();
    (
        image,
        Texture::new_fill(
            Extent3d::new(size.0, size.1, 1),
            TextureDimension::D2,
            &data,
            TextureFormat::Rgba8UnormSrgb,
        ),
    )
}

fn load_from_response(response: ureq::Response) -> (RgbaImage, Texture) {
    let len = response
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_default();
    let mut bytes: Vec<u8> = Vec::with_capacity(len);
    response
        .into_reader()
        .take(len as u64 * 2)
        .read_to_end(&mut bytes)
        .expect("Could not read to end");
    load_from_bytes(&bytes)
}

use std::sync::mpsc::{channel, Receiver, Sender};

use crate::persistence::{Persistable, PersistenceEvent};

fn request_image(url: String, sender: Sender<(String, RgbaImage, Texture)>) -> JoinHandle<()> {
    spawn(move || {
        info!("Requesting image from {}", url);
        let response = ureq::get(&url).call().expect("Failed to request image");
        let (i, t) = load_from_response(response);
        sender
            .send((url, i, t))
            .expect("Failed to send the loaded image");
    })
}

pub struct ImagesPlugin;

impl Plugin for ImagesPlugin {
    fn build(&self, builder: &mut AppBuilder) {
        let (sender, receiver) = channel();
        builder
            .insert_non_send_resource(ImagesRes {
                sender,
                receiver,
                url: "".into(),
                next_egui_id: 0,
            })
            .add_plugin(crate::persistence::PersistencePlugin::<Background>::new())
            .add_event::<BackgroundEvent>()
            .add_system(receive_images.system())
            .add_system(auto_request_images.system())
            .add_system(event_handler.system())
            .add_system(BackgroundEditor::render.system())
            .add_system(images.system());
    }
}

struct ImagesRes {
    url: String,
    sender: Sender<(String, RgbaImage, Texture)>,
    receiver: Receiver<(String, RgbaImage, Texture)>,
    next_egui_id: u64,
}

fn receive_images(
    mut egui_context: ResMut<EguiContext>,
    mut images: NonSendMut<ImagesRes>,
    mut textures: ResMut<Assets<Texture>>,
    mut commands: Commands,
    backgrounds: Query<(Entity, &Background)>,
) {
    let ImagesRes {
        ref mut receiver,
        ref mut next_egui_id,
        ..
    } = *images;
    for (url, image, tex) in receiver.try_iter() {
        let texture_handle = textures.add(tex);
        egui_context.set_egui_texture(*next_egui_id, texture_handle.clone());

        for (e, _bg) in backgrounds.iter().filter(|(_, bg)| bg.url == url) {
            commands.entity(e).insert(BackgroundData {
                image: image.clone(),
                texture_handle: texture_handle.clone(),
                ui_texture: egui::TextureId::User(*next_egui_id),
            });
        }

        *next_egui_id += 1;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Background {
    name: String,
    url: String,
    color_channels: (usize, usize, usize),
}

impl Persistable for Background {
    fn file_path() -> &'static Path {
        Path::new("backgrounds.json")
    }
}

struct BackgroundData {
    image: RgbaImage,
    texture_handle: Handle<Texture>,
    ui_texture: egui::TextureId,
}

struct Requested;

fn auto_request_images(
    backgrounds: Query<(Entity, &Background), Without<Requested>>,
    images: NonSendMut<ImagesRes>,
    mut commands: Commands,
) {
    for (e, bg) in backgrounds.iter() {
        commands.entity(e).insert(Requested);
        request_image(bg.url.clone(), images.sender.clone());
    }
}

fn images(
    egui_context: ResMut<EguiContext>,
    mut images: NonSendMut<ImagesRes>,
    mut commands: Commands,
    backgrounds: Query<(&Background, &BackgroundData)>,
    mut bg_persistence: EventWriter<PersistenceEvent<Background>>,
) {
    egui::Window::new("Download Image").show(egui_context.ctx(), |ui| {
        ui.label("URL:");
        ui.text_edit_singleline(&mut images.url);
        if ui.button("Request").clicked() {
            let e = commands
                .spawn()
                .insert(Background {
                    name: images.url.clone(),
                    url: images.url.clone(),
                    color_channels: (255, 255, 255),
                })
                .id();
        }
        ui.separator();
    });

    egui::Window::new("Images").show(egui_context.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Add New").clicked() {
                bg_persistence.send(PersistenceEvent::FileIn);
            }
            if ui.button("File In").clicked() {
                bg_persistence.send(PersistenceEvent::FileIn);
            }
            if ui.button("File Out").clicked() {
                bg_persistence.send(PersistenceEvent::FileOut);
            }
        });

        for (bg, bgd) in backgrounds.iter() {
            // ui.label(format!(
            //     "size: {:?}, format: {:?}, len: {}",
            //     t.size,
            //     t.format,
            //     t.data.len()
            // ));
            //
            ui.horizontal(|ui| {
                ui.image(bgd.ui_texture, [WIDTH as f32, HEIGHT as f32]);
                if ui.small_button("edit").clicked() {
                    commands.spawn().insert(BackgroundEditor::new_for(&bg.name));
                }
            });
        }
    });
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
enum BackgroundEvent {
    Updated(Background),
}

struct BackgroundEditor {
    target: String,
}

fn event_handler(
    mut events: EventReader<BackgroundEvent>,
    mut query: Query<(Entity, &mut Background)>,
    mut commands: Commands,
) {
    for e in events.iter() {
        info!("{:?}", e);
        match e {
            //BackgroundEvent::Created(slide) => {
            //    commands.spawn().insert(slide.clone());
            //}
            BackgroundEvent::Updated(background) => {
                for (_, mut bg) in query.iter_mut() {
                    if bg.name == background.name {
                        *bg = background.clone();
                    }
                }
            } //BackgroundEvent::Renamed(old_name, new_name) => {
              //    for (_, mut s) in query.iter_mut() {
              //        if s.name == *old_name {
              //            s.name = new_name.clone();
              //        }
              //        for mut a in s.actions.iter_mut() {
              //            if a.target_slide == *old_name {
              //                a.target_slide = new_name.clone();
              //            }
              //        }
              //    }
              //}
              //BackgroundEvent::Deleted(name) => {
              //    for (eid, s) in query.iter_mut() {
              //        if s.name == *name {
              //            commands.entity(eid).despawn();
              //        }
              //    }
              //}
        }
    }
}

impl BackgroundEditor {
    fn new_for(target: &str) -> Self {
        Self {
            target: target.into(),
        }
    }
    fn render(
        egui_context: ResMut<EguiContext>,
        mut editors: Query<(Entity, &mut Self)>,
        backgrounds: Query<(&Background, &BackgroundData)>,
        mut bg_events: EventWriter<BackgroundEvent>,
        mut commands: Commands,
    ) {
        for (editor_id, mut editor) in editors.iter_mut() {
            let (saved, bdata) = match backgrounds
                .iter()
                .filter(|(b, _)| b.name == editor.target)
                .next()
            {
                None => {
                    commands.entity(editor_id).despawn();
                    continue;
                }
                Some(s) => s,
            };
            let mut unsaved = saved.clone();
            let mut open = true;
            egui::Window::new(format!("Edit: {}", saved.name))
                .id(egui::Id::new(editor_id))
                .open(&mut open)
                .show(egui_context.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.label(&unsaved.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("URL:");
                        ui.label(&unsaved.url);
                    });
                    ui.separator();
                    let mut ascii = convert_background_to_ascii(saved, bdata);
                    ui.add(
                        egui::TextEdit::multiline(&mut ascii)
                            .text_style(egui::TextStyle::Monospace)
                            .enabled(true),
                    );
                });
            if !open {
                commands.entity(editor_id).despawn();
            }
            if unsaved != *saved {
                bg_events.send(BackgroundEvent::Updated(unsaved.clone()));
            }
        }
    }
}

fn convert_background_to_ascii(bg: &Background, bgd: &BackgroundData) -> String {
    bgd.image
        .pixels()
        .map(|p| pixel_to_intensity(bg, p))
        .map(|i| intensity_to_ascii(i))
        .collect::<Vec<_>>()
        .chunks(WIDTH as usize)
        .map(|c| c.into_iter().cloned().collect::<String>() + "\n")
        .collect()
}

fn pixel_to_intensity(bg: &Background, p: &image::Rgba<u8>) -> u8 {
    let p = p.0;
    let cc = bg.color_channels;
    let (r, g, b) = (p[0] as usize, p[1] as usize, p[2] as usize);
    (if cc.0 == cc.1 && cc.1 == cc.2 {
        (r + g + b) / 3
    } else {
        (r * cc.0 + g * cc.1 + b * cc.2) / (cc.0 + cc.1 + cc.2)
    }) as u8
}

// Copied from edelsonc/asciify
fn intensity_to_ascii(value: u8) -> &'static str {
    let ascii_chars = [
        " ", ".", "^", ",", ":", "_", "=", "~", "+", "O", "o", "*", "#", "&", "%", "B", "@", "$",
    ];

    let n_chars = ascii_chars.len() as u8;
    let step = 255u8 / n_chars;
    for i in 1..(n_chars - 1) {
        let comp = &step * i;
        if value < comp {
            let idx = (i - 1) as usize;
            return ascii_chars[idx];
        }
    }

    ascii_chars[(n_chars - 1) as usize]
}
