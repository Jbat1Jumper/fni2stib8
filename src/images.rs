use serde::*;
use crate::model::EditorsOpen;
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

use crate::{
    editors::RenameDialog,
    model::CrudEvent,
    persistence::{Persistable, PersistenceEvent},
};

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
            .add_plugin(crate::model::CrudPlugin::<Background>::new())
            .add_system(receive_images.system())
            .add_system(RenameDialog::<Background>::render.system())
            .add_system(auto_request_images.system())
            .add_system(DeleteBgDialog::render.system())
            .add_system(BackgroundEditor::render.system())
            .add_system(BackgroundEditor::handle_renames.system())
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
    fn sortable_name<'a>(&'a self) -> &'a str {
        &self.name
    }
}
impl crate::model::Crudable for Background {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn set_name(&mut self, new_name: String) {
        self.name = new_name;
    }
    fn default_name_prefix() -> &'static str {
        "background"
    }
}

pub struct BackgroundData {
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
    backgrounds: Query<(&Background, Option<&BackgroundData>)>,
    mut bg_events: EventWriter<CrudEvent<Background>>,
    editors_open: Res<EditorsOpen>,
) {
    let valid_bg_names: Vec<_> = backgrounds.iter().map(|(bg, _)| bg.name.clone()).collect();
    if !editors_open.0 {
        return;
    }
    egui::Window::new("Images").show(egui_context.ctx(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Add New").clicked() {
                let new_name = (0..10000)
                    .map(|n| format!("background{}", n))
                    .filter(|name| !valid_bg_names.contains(name))
                    .next()
                    .expect("Abusrd amount of badly named slides");
                bg_events.send(CrudEvent::Created(Background {
                    name: new_name,
                    url: "https://img.freepik.com/free-photo/question-mark-icon-glow-dark-3d-illustration_103740-348.jpg?size=626&ext=jpg".into(),
                    color_channels: (255, 255, 255),
                }));
            }
        });
        ui.separator();

        for (bg, bgd) in backgrounds.iter() {
            // ui.label(format!(
            //     "size: {:?}, format: {:?}, len: {}",
            //     t.size,
            //     t.format,
            //     t.data.len()
            // ));
            //

            ui.horizontal(|ui| {
                ui.label(&bg.name);
                if let Some(bgd) = bgd {
                    ui.image(bgd.ui_texture, [WIDTH as f32, 2. * HEIGHT as f32]);
                }
                if ui.button("edit").clicked() {
                    commands.spawn().insert(BackgroundEditor::new_for(&bg.name));
                }
                if ui.button("rename").clicked() {
                    commands.insert_resource(RenameDialog::new_for(bg));
                }
                if ui.button("delete").clicked() {
                    commands.insert_resource(DeleteBgDialog(bg.name.clone()));
                }
            });
        }
    });
}

struct DeleteBgDialog(String);
impl DeleteBgDialog {
    fn render(
        egui_context: ResMut<EguiContext>,
        dialog: Option<ResMut<Self>>,
        mut slide_events: EventWriter<CrudEvent<Background>>,
        mut commands: Commands,
        slides: Query<&crate::model::Slide>,
        editors_open: Res<EditorsOpen>,
    ) {
        if dialog.is_none() {
            return;
        }
        let mut dialog = dialog.unwrap();

        let slides_with_references: Vec<_> = slides
            .iter()
            .filter(|s| s.background == dialog.0)
            .map(|s| s.name.clone())
            .collect();

        if !editors_open.0 {
            return;
        }
        egui::Window::new("Delete background").show(egui_context.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Goging to delete \"{}\"", dialog.0));
            });
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<Self>();
                }
                if slides_with_references.is_empty() {
                    if ui.button("Delete").clicked() {
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct BackgroundEditor {
    target: String,
}

impl BackgroundEditor {
    fn new_for(target: &str) -> Self {
        Self {
            target: target.into(),
        }
    }
    fn handle_renames(
        mut editors: Query<&mut Self>,
        mut slide_events: EventReader<CrudEvent<Background>>,
    ) {
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
        backgrounds: Query<(Entity, &Background, &BackgroundData)>,
        mut bg_events: EventWriter<CrudEvent<Background>>,
        mut commands: Commands,
        editors_open: Res<EditorsOpen>,
    ) {
        for (editor_id, mut editor) in editors.iter_mut() {
            let (bg_entity, saved, bdata) = match backgrounds
                .iter()
                .filter(|(_, b, _)| b.name == editor.target)
                .next()
            {
                None => {
                    commands.entity(editor_id).despawn();
                    continue;
                }
                Some(s) => s,
            };
            if !editors_open.0 {
                return;
            }
            let mut unsaved = saved.clone();
            let mut open = true;
            egui::Window::new(format!("Edit: {}", saved.name))
                .default_width(966.)
                .min_width(966.)
                .id(egui::Id::new(editor_id))
                .open(&mut open)
                .show(egui_context.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.label(&unsaved.name);
                        if ui.small_button("rename").clicked() {
                            commands.insert_resource(RenameDialog::new_for(&unsaved));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("URL:");
                        ui.text_edit_multiline(&mut unsaved.url);
                        if ui.small_button("refresh").clicked() {
                            commands.entity(bg_entity).remove::<Requested>();
                        }
                    });
                    ui.separator();
                    let mut ascii = convert_background_to_ascii(saved, bdata, 1.0);
                    ui.add(
                        egui::TextEdit::multiline(&mut ascii)
                            .text_style(egui::TextStyle::Monospace)
                            .enabled(false),
                    );
                });
            if !open {
                commands.entity(editor_id).despawn();
            }
            if unsaved != *saved {
                bg_events.send(CrudEvent::Updated(unsaved.clone()));
            }
        }
    }
}

pub fn convert_background_to_ascii(bg: &Background, bgd: &BackgroundData, alpha: f32) -> String {
    info!("Converting {} to ascii", bg.name);
    bgd.image
        .pixels()
        .map(|p| pixel_to_intensity(bg, p))
        .map(|i| (i as f32 * alpha) as u8)
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
        ".", "^", ",", ":", "_", "=", "~", "+", "O", "o", "*", "#", "&", "%", "B", "@", "$",
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
