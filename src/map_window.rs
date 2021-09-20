use std::error::Error;

use egui::CtxRef;
use miniquad as mq;

use crate::{
    blacklist::{Blacklist, BlacklistReason},
    bsp::BSPLump,
    kv::{self, Entity},
    lump_helper,
    platform::{self, file_picker, save_picker},
    vtf::VTFHEADER,
};

#[derive(Debug)]
pub struct MapWindowOffscreen {
    pub color_img: mq::Texture,
    pub depth_img: mq::Texture,

    pub pass: mq::RenderPass,
    pub col_id: u32,
}

impl MapWindowOffscreen {
    pub fn new(ctx: &mut mq::Context, width: u32, height: u32) -> Self {
        let color_img = mq::Texture::new_render_texture(
            ctx,
            mq::TextureParams {
                width,
                height,
                format: mq::TextureFormat::RGBA8,
                ..Default::default()
            },
        );
        let col_id = color_img.gl_internal_id();

        let depth_img = mq::Texture::new_render_texture(
            ctx,
            mq::TextureParams {
                width,
                height,
                format: mq::TextureFormat::Depth,
                ..Default::default()
            },
        );

        let pass = mq::RenderPass::new(ctx, color_img, depth_img);

        Self {
            color_img,
            col_id,
            depth_img,
            pass,
        }
    }
}

#[derive(Debug)]
pub enum TextureProblem {
    Blacklist(BlacklistReason),
    UnsupportedImageFormat(vtf::ImageFormat), // ?
}

#[derive(Debug)]
pub struct Texture {
    pub texture: mq::Texture,
    pub name: String,

    pub to_remove: bool,

    pub problem: Option<TextureProblem>,
}

#[derive(Debug)]
pub struct MapWindowStage {
    pub name: String,
    pub offscreen: MapWindowOffscreen,

    pub parsed_map: Option<crate::bsp::ParsedMap>,
    pub textures: Vec<Texture>,

    pub entities: Vec<Entity>, // TODO: allow for proper K-V editor
    pub current_entity: usize,

    pub open: bool,

    pub blacklisted_texture: bool,
    pub blacklisted_file: bool,

    pub file_filter: String,
    pub texture_filter: String,
    pub entity_filter: String,
}

impl MapWindowStage {
    pub fn new(
        name: String,
        buf: Vec<u8>,
        ctx: &mut mq::Context,
        width: u32,
        height: u32,
        blacklist: Option<&Blacklist>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut parsed_map = crate::bsp::ParsedMap::new(buf)?;

        let pak = lump_helper!(&mut parsed_map.lumps[40], crate::bsp::BSPLump::PakFile(v) => v);
        let file =
            &parsed_map.buf[pak.base.offset as usize..(pak.base.offset + pak.base.size) as usize];

        if let Some(blacklist) = blacklist {
            for i in &mut pak.files {
                if let Some(v) = blacklist.check(i.data(file)) {
                    i.blacklisted = Some(v);
                }
            }
        }

        let textures = pak
            .files
            .iter()
            .filter(|pakfile| {
                let name = unsafe {
                    std::str::from_utf8_unchecked(
                        &file[pakfile.name.0 as usize..(pakfile.name.0 + pakfile.name.1) as usize],
                    )
                };
                name.ends_with(".vtf")
            })
            .map(|pakfile| {
                let file_data = pakfile.data(file);
                // this library sucks... I needed to fork it just to make it work properly cuz no one tests when accepts PRs...
                let mut data_vec = file_data.to_vec();
                let vtf = vtf::from_bytes(&mut data_vec).unwrap();
                let image = vtf.highres_image;

                let name = pakfile.name(file);

                #[cfg(debug_assertions)]
                println!(
                    "{} | {:#?} | {}x{}",
                    name, image.format, image.width, image.height
                );

                // This library is so bad: UnsupportedImageFormat(Rgba16161616f)
                if let Ok(decoded) = image.decode(0) {
                    Texture {
                        texture: mq::Texture::from_rgba8(
                            ctx,
                            image.width,
                            image.height,
                            decoded.into_rgba8().to_vec().as_slice(),
                        ),
                        name: name.to_string(),
                        to_remove: false,

                        problem: if let Some(reason) = &pakfile.blacklisted {
                            Some(TextureProblem::Blacklist(reason.clone()))
                        } else {
                            None
                        },
                    }
                } else {
                    match image.format {
                        vtf::ImageFormat::Rgba16161616f => {
                            let image_data = image.get_frame(0).unwrap();
                            // RGBA8
                            let mut bytes = vec![0u8; image_data.len() / 2];
                            for i in 0..(image_data.len() / 2) {
                                let slice = &image_data[i * 2..2 + (i * 2)];
                                let array = [slice[0], slice[1]];
                                let chan = half::f16::from_le_bytes(array);
                                // println!("{}", chan);
                                bytes[i] = (chan.to_f32().max(0.0).min(1.0) * 255.0) as u8;
                            }
                            Texture {
                                texture: mq::Texture::from_rgba8(
                                    ctx,
                                    image.width,
                                    image.height,
                                    &bytes,
                                ),
                                name: name.to_string(),
                                to_remove: false,

                                problem: if let Some(reason) = &pakfile.blacklisted {
                                    Some(TextureProblem::Blacklist(reason.clone()))
                                } else {
                                    None
                                },
                            }
                        }
                        vtf::ImageFormat::Abgr8888 => {
                            let mut image_data = image.get_frame(0).unwrap().to_vec();
                            for idx in 0..image_data.len() / 4 {
                                let i = idx * 4;
                                if let [a, b, c, d] = image_data[i..i + 4] {
                                    image_data[i..i + 4].copy_from_slice(&[d, c, b, a]);
                                } else {
                                    unreachable!()
                                }
                            }
                            Texture {
                                texture: mq::Texture::from_rgba8(
                                    ctx,
                                    image.width,
                                    image.height,
                                    &image_data,
                                ),
                                name: name.to_string(),
                                to_remove: false,

                                problem: if let Some(reason) = &pakfile.blacklisted {
                                    Some(TextureProblem::Blacklist(reason.clone()))
                                } else {
                                    None
                                },
                            }
                        }
                        _ => {
                            // TODO...
                            eprintln!("Failed... {}x{}", image.width, image.height);
                            Texture {
                                texture: unsafe { mq::Texture::from_raw_id(0) },
                                name: name.to_string(),
                                to_remove: false,

                                problem: if let Some(reason) = &pakfile.blacklisted {
                                    Some(TextureProblem::Blacklist(reason.clone()))
                                } else {
                                    Some(TextureProblem::UnsupportedImageFormat(image.format))
                                },
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        let entities = kv::parse_ents_hacky(
            lump_helper!(&parsed_map.lumps[0], BSPLump::Entities(v) => v)
                .string
                .as_str(),
        );

        Ok(Self {
            name,
            offscreen: MapWindowOffscreen::new(ctx, width, height),

            parsed_map: Some(parsed_map),
            textures,

            entities,
            current_entity: 0,

            open: true,

            blacklisted_texture: false,
            blacklisted_file: false,

            file_filter: "".to_string(),
            texture_filter: "".to_string(),
            entity_filter: "".to_string(),
        })
    }

    pub fn render_map(&mut self, ctx: &mut mq::Context) {
        /*if !self.open {
            return;
        }*/

        let Self { parsed_map, .. } = self;

        ctx.begin_pass(
            self.offscreen.pass,
            mq::PassAction::clear_color(1.0, 1.0, 1.0, 1.),
        );
        if let Some(parsed_map) = parsed_map {
            // TODO: render map some time soon?
        }
        ctx.end_render_pass()
    }

    pub fn ui(&mut self, egui_ctx: &CtxRef) -> bool {
        let mut grabbed = false;

        if self.open {
            // TODO: split logic at least into multiple different functions...

            let offscreen = egui::TextureId::User(self.offscreen.col_id as u64);
            egui::Window::new(format!("[{}] Map view", self.name))
                .resizable(true)
                .collapsible(true)
                .open(&mut self.open)
                .default_width(256.0)
                .show(egui_ctx, |ui| {
                    if ui
                        .add(egui::ImageButton::new(offscreen, [256.0; 2]))
                        .clicked()
                    {
                        grabbed = true;
                    }

                    ui.allocate_space(egui::vec2(0.0, 0.0));
                });

            egui::Window::new(format!("[{}] Texture view", self.name))
                .resizable(true)
                .scroll(true)
                .default_width(712.0)
                .show(egui_ctx, |ui| {
                    ui.checkbox(&mut self.blacklisted_texture, "Show only blacklisted");
                    let filter = &mut self.texture_filter;
                    ui.horizontal(|ui| {
                        ui.label("Search");
                        ui.text_edit_singleline(filter);
                    });
                    for texture in &mut self.textures {
                        // ui.add(egui::ImageButton::new(
                        //     egui::TextureId::User(texture.gl_internal_id() as u64),
                        //     [256.0; 2],
                        // ));
                        if self.blacklisted_texture {
                            if texture.problem.is_none() {
                                continue;
                            }
                        }
                        if filter.len() > 0 {
                            if texture.name.find(filter.as_str()).is_none() {
                                continue;
                            }
                        }
                        ui.horizontal(|ui| {
                            let size = [
                                texture.texture.width.max(128).min(256) as f32,
                                texture.texture.height.max(128).min(256) as f32,
                            ];
                            ui.image(
                                egui::TextureId::User(texture.texture.gl_internal_id() as u64),
                                size,
                            );
                            ui.vertical(|ui| {
                                ui.label(&texture.name);
                                ui.label(format!(
                                    "{}x{}",
                                    texture.texture.width, texture.texture.height
                                ));
                                if let Some(problem) = &texture.problem {
                                    ui.colored_label(
                                        egui::color::Color32::RED,
                                        match problem {
                                            TextureProblem::Blacklist(v) => {
                                                format!("Blacklisted: {:?}", v)
                                            }
                                            TextureProblem::UnsupportedImageFormat(f) => {
                                                format!("Unsupported image format: {:?}", f)
                                            }
                                        },
                                    );
                                }
                                ui.checkbox(&mut texture.to_remove, "Remove");
                            });
                        });
                    }
                });
            let blacklisted_file_mut = &mut self.blacklisted_file;
            let filter = &mut self.file_filter;
            if let Some(parsed_map) = self.parsed_map.as_mut() {
                egui::Window::new(format!("[{}] ZIP file view", self.name))
                    .resizable(true)
                    .scroll(true)
                    .default_width(712.0)
                    .show(egui_ctx, |ui| {
                        let paklump =
                            lump_helper!(&mut parsed_map.lumps[40], BSPLump::PakFile(v) => v);
                        let pak = paklump.data(&parsed_map.buf);

                        if ui.button("Export ZIP").clicked() {
                            platform::save_picker_zip(pak); // TODO: error handling?
                        }
                        ui.checkbox(blacklisted_file_mut, "Show only blacklisted");
                        ui.horizontal(|ui| {
                            ui.label("Search");
                            ui.text_edit_singleline(filter);
                        });
                        // TODO: import somehow
                        ui.label("Tick to remove it from the PK lump");

                        for pakfile in &mut paklump.files {
                            if *blacklisted_file_mut {
                                if pakfile.blacklisted.is_none() {
                                    continue;
                                }
                            }
                            let name = pakfile.name(pak);
                            if filter.len() > 0 {
                                if name.find(filter.as_str()).is_none() {
                                    continue;
                                }
                            }
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut pakfile.remove, name);
                                if let Some(v) = &pakfile.blacklisted {
                                    ui.colored_label(
                                        egui::color::Color32::RED,
                                        format!(" {:?}", &v),
                                    );
                                }
                            });
                        }
                    });

                if self.entities.len() > 0 {
                    let entities = &mut self.entities;
                    let mutref = &mut self.current_entity;
                    let ents_len = entities.len();
                    let filter = &mut self.entity_filter;
                    const VDF_EXT: [&str; 3] = ["txt", "kv", "vdf"];
                    const VDF_FLT: &str = "KeyValue";
                    egui::Window::new(format!("[{}] Entity view", self.name))
                        .resizable(true)
                        .scroll(true)
                        .default_width(512.0)
                        .show(egui_ctx, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Save whole original KV").clicked() {
                                    let data = lump_helper!(&parsed_map.lumps[0], BSPLump::Entities(v) => v).string.as_bytes();
                                    save_picker(VDF_FLT, &VDF_EXT, data);
                                }
                                if ui.button("Save modified KV to file").clicked() {
                                    let data = entities.iter().map(|f| f.string.as_str()).collect::<Vec<&str>>().join("\n");
                                    save_picker(VDF_FLT, &VDF_EXT, data.as_bytes());
                                }
                            });
                            if ui.button("Replace KV from file").clicked() {
                                if let Some((_, data)) = file_picker(VDF_FLT, &VDF_EXT) {
                                    *entities = kv::parse_ents_hacky(unsafe {
                                        std::str::from_utf8_unchecked(&data)
                                    });
                                    *mutref = 0;
                                }
                            }
                            if ui.button("Reset entities (No confirmation)").clicked() {
                                *mutref = 0;
                                *entities = kv::parse_ents_hacky(lump_helper!(&parsed_map.lumps[0], BSPLump::Entities(v) => v).string.as_str());
                            }
                            ui.horizontal(|ui| {
                                ui.label("Search");
                                ui.text_edit_singleline(filter);
                            });
                            egui::ComboBox::from_label("Entity")
                                .selected_text(format!("{}: {}{}", *mutref, entities[*mutref].pretty_name(), if entities[*mutref].dirty {
                                    " *"
                                } else {
                                    ""
                                }))
                                .width(512.0)
                                .show_ui(ui, |ui| {
                                    for i in 0..ents_len {
                                        if filter.len() > 0 {
                                            if entities[i].string.find(filter.as_str()).is_none() {
                                                continue;
                                            }
                                        }
                                        ui.selectable_value(mutref, i, format!("{}: {}{}", i, entities[i].pretty_name(), if entities[i].dirty {
                                            " *"
                                        } else {
                                            ""
                                        }));
                                    }
                                }
                            );

                            if ui.code_editor(&mut entities[*mutref].string).changed() {
                                entities[*mutref].dirty = true;
                            }
                        });
                }
            }
        }

        return grabbed;
    }
}
