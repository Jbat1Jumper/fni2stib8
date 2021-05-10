use std::{
    io::Read,
    thread::{spawn, JoinHandle},
};

use bevy::{
    prelude::*,
    render::texture::{Extent3d, ImageType, TextureDimension, TextureFormat},
};
use bevy_egui::{egui, EguiContext};
use image::GenericImageView;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128 / 2;

fn load_from_bytes(bytes: &[u8]) -> Texture {
    let reader = image::io::Reader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .expect("Cursor never fails");
    let image = reader
        .decode()
        .expect("Failed to decode image")
        .resize_to_fill(WIDTH, HEIGHT, image::imageops::FilterType::Nearest);
    let data: Vec<u8> = image
        .to_rgba8()
        .pixels()
        .map(|p| &p.0)
        .flatten()
        .cloned()
        .collect();
    Texture::new_fill(
        Extent3d::new(image.dimensions().0, image.dimensions().1, 1),
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
    )
}

fn load_from_response(response: ureq::Response) -> Texture {
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

fn request_image(url: String, sender: Sender<Texture>) -> JoinHandle<()> {
    spawn(move || {
        let response = ureq::get(&url).call().expect("Failed to request image");
        sender
            .send(load_from_response(response))
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
                received: vec![],
            })
            .add_system(test.system());
    }
}

struct ImagesRes {
    url: String,
    sender: Sender<Texture>,
    receiver: Receiver<Texture>,
    received: Vec<Texture>,
}

fn test(egui_context: ResMut<EguiContext>, mut images: NonSendMut<ImagesRes>) {
    egui::Window::new("Test images").show(egui_context.ctx(), |ui| {
        ui.label("URL:");
        ui.text_edit_singleline(&mut images.url);
        if ui.button("Request").clicked() {
            request_image(images.url.clone(), images.sender.clone());
        }
        ui.separator();

        {
            let ImagesRes {
                ref mut receiver,
                ref mut received,
                ..
            } = *images;
            for t in receiver.try_iter() {
                received.push(t);
            }
            for t in received.iter() {
                ui.label(format!(
                    "size: {:?}, format: {:?}, len: {}",
                    t.size,
                    t.format,
                    t.data.len()
                ));
            }
        }
    });
}
