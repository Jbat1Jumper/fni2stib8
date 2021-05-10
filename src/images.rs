use std::{
    io::Read,
    thread::{spawn, JoinHandle},
};

use bevy::{
    prelude::*,
    render::texture::{Extent3d, ImageType, TextureDimension, TextureFormat},
};
use bevy_egui::{egui, EguiContext};
use image::{GenericImageView, RgbaImage};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128 / 2;

fn load_from_bytes(bytes: &[u8]) -> (RgbaImage, Texture) {
    let reader = image::io::Reader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .expect("Cursor never fails");
    let image = reader
        .decode()
        .expect("Failed to decode image")
        .resize_to_fill(WIDTH, HEIGHT, image::imageops::FilterType::Nearest)
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

fn request_image(url: String, sender: Sender<(String, RgbaImage, Texture)>) -> JoinHandle<()> {
    spawn(move || {
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
            .add_system(receive_images.system())
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

struct Background {
    name: String,
    url: String,
}
struct BackgroundData {
    image: RgbaImage,
    texture_handle: Handle<Texture>,
    ui_texture: egui::TextureId,
}

fn images(
    egui_context: ResMut<EguiContext>,
    mut images: NonSendMut<ImagesRes>,
    mut commands: Commands,
    backgrounds: Query<(&Background, &BackgroundData)>,
) {
    egui::Window::new("Download Image").show(egui_context.ctx(), |ui| {
        ui.label("URL:");
        ui.text_edit_singleline(&mut images.url);
        if ui.button("Request").clicked() {
            request_image(images.url.clone(), images.sender.clone());
            let e = commands
                .spawn()
                .insert(Background {
                    name: images.url.clone(),
                    url: images.url.clone(),
                })
                .id();
        }
        ui.separator();
    });

    egui::Window::new("Images").show(egui_context.ctx(), |ui| {
        for (bg, bgd) in backgrounds.iter() {
            // ui.label(format!(
            //     "size: {:?}, format: {:?}, len: {}",
            //     t.size,
            //     t.format,
            //     t.data.len()
            // ));

            ui.image(bgd.ui_texture, [WIDTH as f32, HEIGHT as f32]);
        }
    });
}
