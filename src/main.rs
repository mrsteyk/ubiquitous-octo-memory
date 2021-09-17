use std::{cell::RefCell, io::Write, ops::Add, rc::Rc};

use {egui_miniquad as egui_mq, miniquad as mq};

mod blacklist;
mod bsp;
mod kv;
mod platform;
mod vtf;

mod map_window;
use bsp::BSPLump;
use map_window::*;

struct Stage {
    egui_mq: egui_mq::EguiMq,

    maps: Vec<Rc<RefCell<MapWindowStage>>>,
    current_capture: Option<Rc<RefCell<MapWindowStage>>>,

    blacklist: Option<blacklist::Blacklist>,
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    pub fn console_log(msg: *const ::std::os::raw::c_char);
}

impl Stage {
    fn new(ctx: &mut mq::Context) -> Self {
        Self {
            egui_mq: egui_mq::EguiMq::new(ctx),

            maps: vec![],
            current_capture: None,

            blacklist: None,
        }
    }

    fn ui(&mut self, ctx: &mut mq::Context) {
        let Self { egui_mq, .. } = self;

        let egui_ctx = egui_mq.egui_ctx();

        let maps = &mut self.maps;
        let blacklist_mut = &mut self.blacklist;

        egui::TopBottomPanel::top("main_menu_bar").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Open").clicked() {
                        // Open
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(map_data) = platform::file_picker_map() {
                            let (stem, vec) = map_data;

                            #[cfg(debug_assertions)]
                            {
                                println!("stem: {}", stem);
                            }

                            match MapWindowStage::new(stem, vec, ctx, 256, 256, blacklist_mut.as_ref()) {
                                Ok(v) =>{
                                    #[cfg(debug_assertions)]
                                    if let Some(v) = &v.parsed_map {
                                        // let ents = lump_helper!(&v.lumps[0], bsp::BSPLump::Entities(v) => v);
                                        // println!("{}", &ents.string);

                                        let pak = lump_helper!(&v.lumps[40], bsp::BSPLump::PakFile(v) => v);
                                        let file = &v.buf[pak.base.offset as usize..(pak.base.offset + pak.base.size) as usize];
                                        for pakfile in &pak.files {
                                            let name = pakfile.name(file);
                                            println!("{}: {:08X?}", name, pakfile.data);
                                        }

                                        println!("Order: {:?}", v.order);
                                        println!("Iteration: {}", v.iteration);
                                    };
                                    maps.push(Rc::new(RefCell::new(v)))
                                },
                                Err(v) => {
                                    println!("{:#?}", v);
                                }
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            platform::wasm_file_picker(maps as *mut std::vec::Vec<Rc<RefCell<map_window::MapWindowStage>>>, blacklist_mut as *mut Option<blacklist::Blacklist>, ctx as *mut mq::Context);
                            unsafe {
                                platform::console_log(std::ffi::CString::new(format!("{}", maps as *mut _ as u32)).unwrap().as_ptr());
                            };
                        }
                    }
                    if ui.button("Load blacklist JSON").clicked() {
                        if let Some((_, data)) = platform::file_picker_json() {
                            if let Ok(blacklist) = blacklist::Blacklist::new(&data) {
                                *blacklist_mut = Some(blacklist);
                            }
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if ui.button("Quit").clicked() {
                            std::process::exit(0);
                        }
                    }
                });

                if let Some(remove_index) = {
                    let mut ret: Option<usize> = None;

                    for i in 0..maps.len() {
                        let map_rc = &maps[i];

                        let mut map = map_rc.borrow_mut();

                        let title = &map.name;
                        let parsed_map = &map.parsed_map;
                        let entities = &map.entities;
                        let textures = &map.textures;

                        let mut show = map.open; // hack to work around borrow problems

                        egui::menu::menu(ui, title, |ui| {
                            ui.checkbox(&mut show, "Views");
                            if ui.button("Save").clicked() {
                                // brih...
                                if let Some(parsed_map) = parsed_map {
                                    // was used when trying to properly serialise the map...
                                    // let total_lump_sizes = parsed_map.lumps.as_ref().iter().map(|f| match f {
                                    //     BSPLump::Entities(v) => v.base.size,
                                    //     BSPLump::PakFile(v) => v.base.size,
                                    //     BSPLump::Unknown(v) => v.size,
                                    //     _ => unreachable!(),
                                    // }).reduce(|a, b| a+b).unwrap();

                                    let entlump = lump_helper!(&parsed_map.lumps[0], BSPLump::Entities(v) => v);
                                    let entity_data_str = entities.iter().map(|f| f.string.as_str()).collect::<Vec<&str>>().join("\n").add("\n\0");
                                    let (entity_data, ent_vec) = if entities.iter().find(|f| f.dirty).is_some() {
                                        if entlump.base.four == 0 {
                                            (entity_data_str.as_bytes(), None)
                                        } else {
                                            // we have compressed data :/
                                            let bytes = entity_data_str.as_bytes();

                                            let output = gmod_lzma::compress_valve(bytes, 6).unwrap();

                                            let len = (output.len() - 5) as u32; // remove LZMA props size...
                                            let output = [&[0x4Cu8, 0x5A, 0x4D, 0x41] as &[u8], &(bytes.len() as u32).to_le_bytes(), &len.to_le_bytes(), output.as_slice()].concat();

                                            (&[] as &[u8], Some(output))
                                        }
                                    } else {
                                        (&parsed_map.buf[entlump.base.offset as usize..(entlump.base.offset + entlump.base.size) as usize], None)
                                    };

                                    let entity_data = if let Some(v) = ent_vec.as_ref() {
                                        v.as_slice()
                                    } else {
                                        entity_data
                                    };

                                    // TODO: recreate array
                                    // TODO: remove material?
                                    let paklump = lump_helper!(&parsed_map.lumps[40], BSPLump::PakFile(v) => v);
                                    let pakbuf = paklump.data(&parsed_map.buf);
                                    let mut pakfiles = paklump.files.clone();
                                    for texture in textures {
                                        if texture.to_remove {
                                            let name = texture.name.as_str();
                                            if let Some(v) = pakfiles.iter_mut().find(|f| {
                                                f.name(pakbuf).eq(name)
                                            }) {
                                                v.remove = true;
                                            }
                                        }
                                    }
                                    // TODO: REFACTOR INTO A FUNCTION!
                                    let (mut zip_data, lzma_pass) = if pakfiles.iter().find(|f| f.remove).is_some() {
                                        let zipw = Vec::<u8>::new();
                                        let zipc = std::io::Cursor::new(zipw);
                                        let mut zip_writer = zip::ZipWriter::new(zipc);
                                        let options = zip::write::FileOptions::default()
                                            .compression_method(zip::CompressionMethod::Stored) // force store to be extra safe
                                            .unix_permissions(0o755);
                                        let mut lzma_pass = false;
                                        for pakfile in &pakfiles {
                                            if !pakfile.remove {
                                                zip_writer.start_file(pakfile.name(pakbuf), options).unwrap();
                                                match pakfile.compression_algo {
                                                    bsp::PakAlgo::None => {
                                                        zip_writer.write_all(pakfile.data(pakbuf)).unwrap();
                                                    },
                                                    // We care for compressed size because I don't want to go over it if I just recompress...
                                                    bsp::PakAlgo::LZMA(comp, _) => {
                                                        let data = pakfile.data(&[]);

                                                        lzma_pass = true;

                                                        let comp_data = pakfile.data;
                                                        zip_writer.write_all(&pakbuf[comp_data.0 as usize..(comp_data.0+comp_data.1) as usize]).unwrap();

                                                        // for future?
                                                        // let output = gmod_lzma::compress_valve(data, 9 | (1 << 31)).unwrap();

                                                        // eprintln!("LZMA comp: {} V {}", comp, output.len());
                                                        // #[cfg(debug_assertions)]
                                                        // assert!(comp as usize >= output.len());

                                                        // // Version + props size as per LZMA in ZIP spec
                                                        // zip_writer.write_all(&[0x09, 0x26, 5, 0]).unwrap();
                                                        // zip_writer.write_all(output.as_slice()).unwrap();
                                                    }
                                                }
                                            }
                                        }
                                        (zip_writer.finish().unwrap(), lzma_pass)
                                    } else {
                                        (std::io::Cursor::new(pakbuf.to_vec()), false)
                                    };
                                    let pak_data_mut = zip_data.get_mut().as_mut_slice();
                                    // OMG why
                                    if lzma_pass {
                                        let mut position = 0usize;
                                        loop {
                                            let header_pos = position;
                                            let header = &pak_data_mut[position..position + 30];
                                            position += 30;
                                            match &header[0..4] {
                                                &[0x50, 0x4B, 3, 4] => {
                                                    let compressed_size =
                                                        unsafe { *((&header[18..22]).as_ptr().cast::<u32>()) };
                                                    // let data_size =
                                                    //     unsafe { *((&header[22..26]).as_ptr().cast::<u32>()) };
                                                    let name_size =
                                                        unsafe { *((&header[26..28]).as_ptr().cast::<u16>()) };
                                                    let extra_size =
                                                        unsafe { *((&header[28..30]).as_ptr().cast::<u16>()) };
                                                    let name = (position as u32, name_size as u32);
                                                    position += name_size as usize;
                                                    position += extra_size as usize;
                                                    position += compressed_size as usize;
                                                    for pakfile in &pakfiles {
                                                        if !pakfile.remove {
                                                            match pakfile.compression_algo {
                                                                bsp::PakAlgo::LZMA(_, _) => {
                                                                    if pakfile.name(pakbuf).as_bytes() == &pak_data_mut[name.0 as usize..(name.0 + name.1) as usize] {
                                                                        // we got a hit for LZMA packed file, yay?
                                                                        pak_data_mut[header_pos + 8] = 0xE;
                                                                        pak_data_mut[header_pos + 22..header_pos + 26].copy_from_slice(&(pakfile.data(&[]).len() as u32).to_le_bytes());
                                                                    }
                                                                },
                                                                _ => {
                                                                    // do nothing...
                                                                }
                                                            }
                                                        }
                                                    }
                                                },
                                                &[0x50, 0x4B, 1, 2] => {
                                                    break; // Central directory aka ending stuff
                                                }
                                                _ => {
                                                    unreachable!();
                                                }
                                            }
                                        }
                                    }
                                    let pak_data = pak_data_mut.as_ref();

                                    let mut lumps_temp = [0u32; 64]; // offset
                                    {
                                        let mut accum = 0x40c;
                                        for (i, _) in &parsed_map.order {
                                            lumps_temp[*i as usize] = accum;
                                            match *i {
                                                0 => {
                                                    accum += entity_data.len() as u32;
                                                }
                                                40 => {
                                                    accum += pak_data.len() as u32;
                                                }
                                                v => {
                                                    let sz = lump_helper!(&parsed_map.lumps[v as usize], BSPLump::Unknown(l) => l).size;
                                                    accum += sz;
                                                    if sz == 0 {
                                                        lumps_temp[v as usize] = 0;
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // This code is broken because I break offsets a lot of things rely on...
                                    // let mut buf = Vec::<u8>::with_capacity(0x40c + total_lump_sizes as usize);
                                    // buf.extend_from_slice(b"VBSP");
                                    // buf.extend_from_slice(&parsed_map.version.to_le_bytes());
                                    // for lump in 0..64usize {
                                    //     // offset
                                    //     buf.extend_from_slice(&lumps_temp[lump].to_le_bytes());
                                    //     match lump {
                                    //         0 => {
                                    //             // size
                                    //             buf.extend_from_slice(&(entity_data.len() as u32).to_le_bytes());

                                    //             let base = &lump_helper!(&parsed_map.lumps[0], BSPLump::Entities(v) => v).base;
                                    //             buf.extend_from_slice(&base.version.to_le_bytes());
                                    //         }
                                    //         40 => {
                                    //             // size
                                    //             buf.extend_from_slice(&(pak_data.len() as u32).to_le_bytes());

                                    //             let base = &lump_helper!(&parsed_map.lumps[40], BSPLump::PakFile(v) => v).base;
                                    //             buf.extend_from_slice(&base.version.to_le_bytes());
                                    //         }
                                    //         i => {
                                    //             let base = lump_helper!(&parsed_map.lumps[i], BSPLump::Unknown(v) => v);
                                    //             buf.extend_from_slice(&base.size.to_le_bytes());
                                    //             buf.extend_from_slice(&base.version.to_le_bytes());
                                    //         }
                                    //     }
                                    //     // ex fourCC aka XBox's compressed size stuff
                                    //     buf.extend_from_slice(&[0,0,0,0]);
                                    // }
                                    // buf.extend_from_slice(&parsed_map.iteration.to_le_bytes());

                                    // #[cfg(debug_assertions)]
                                    // assert_eq!(buf.len(), 0x40c);

                                    // // At this point header is written, write all lump data...
                                    // for (i, _) in &parsed_map.order {
                                    //     match *i {
                                    //         0 => buf.extend_from_slice(entity_data),
                                    //         40 => buf.extend_from_slice(pak_data),
                                    //         i => {
                                    //             let base = lump_helper!(&parsed_map.lumps[i as usize], BSPLump::Unknown(v) => v);
                                    //             if base.size > 0 {
                                    //                 let data = &parsed_map.buf[base.offset as usize..(base.offset + base.size) as usize];
                                    //                 buf.extend_from_slice(data);
                                    //             }
                                    //         }
                                    //     }
                                    // }

                                    // can we serialise(replace) stuff back?
                                    // sadly, this is a limitation and user can't even know if something's wrong
                                    let can_ent = (entity_data.len() as u32) <= entlump.base.size;
                                    let can_pak = (pak_data.len() as u32) <= paklump.base.size;
                                    if can_ent && can_pak {
                                        let mut buf = parsed_map.buf.clone();
                                        let ent_sz = entity_data.len() as u32;
                                        let pak_sz = pak_data.len() as u32;

                                        buf[entlump.base.offset as usize..(entlump.base.offset+ent_sz) as usize].copy_from_slice(entity_data);
                                        if ent_sz < entlump.base.size {
                                            for i in 0..(entlump.base.size-ent_sz) {
                                                buf[(entlump.base.offset+ent_sz+i) as usize] = 0;
                                            }
                                        }

                                        buf[paklump.base.offset as usize..(paklump.base.offset+pak_sz) as usize].copy_from_slice(pak_data);
                                        if pak_sz < paklump.base.size {
                                            for i in 0..(paklump.base.size-pak_sz) {
                                                buf[(paklump.base.offset+pak_sz+i) as usize] = 0;
                                            }
                                        }

                                        platform::save_picker(&title, &["bsp"], &buf);
                                    } else {
                                        eprintln!("Failed: {} ({} V {}) | {} ({} V {})", can_ent, (entity_data.len() as u32), entlump.base.size, can_pak, (pak_data.len() as u32), paklump.base.size);
                                    }
                                }
                            }
                            if ui.button("Close").clicked() {
                                ret = Some(i);
                            }
                        });

                        map.open = show;
                    }

                    ret
                } {
                    let a = &maps[remove_index];
                    for i in &a.borrow().textures {
                        if i.texture.gl_internal_id() != 0 {
                            i.texture.delete()
                        }
                    }

                    maps.remove(remove_index);
                };
            });
        });

        for map in &self.maps {
            if map.borrow_mut().ui(&egui_ctx) {
                if self.current_capture.is_none() {
                    self.current_capture = Some(map.clone());

                    ctx.set_cursor_grab(true);
                    ctx.show_mouse(false);

                    println!("capturing on window {}", map.borrow_mut().name);
                }
            }
        }
    }
}

impl mq::EventHandler for Stage {
    fn update(&mut self, _ctx: &mut mq::Context) {}

    fn draw(&mut self, ctx: &mut mq::Context) {
        ctx.clear(Some((1., 1., 1., 1.)), None, None);
        ctx.begin_default_pass(mq::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        ctx.end_render_pass();

        self.egui_mq.begin_frame(ctx);
        self.ui(ctx);
        self.egui_mq.end_frame(ctx);

        // Draw things behind egui here
        // offscreen, before UI draw to update RT textures
        for map in &self.maps {
            map.borrow_mut().render_map(ctx);
        }

        self.egui_mq.draw(ctx);

        // Draw things in front of egui here

        ctx.commit_frame();
    }

    fn mouse_motion_event(&mut self, ctx: &mut mq::Context, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(ctx, x, y);
    }

    fn mouse_wheel_event(&mut self, ctx: &mut mq::Context, dx: f32, dy: f32) {
        self.egui_mq.mouse_wheel_event(ctx, dx, dy);
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut mq::Context,
        mb: mq::MouseButton,
        x: f32,
        y: f32,
    ) {
        self.egui_mq.mouse_button_down_event(ctx, mb, x, y);
    }

    fn mouse_button_up_event(
        &mut self,
        ctx: &mut mq::Context,
        mb: mq::MouseButton,
        x: f32,
        y: f32,
    ) {
        self.egui_mq.mouse_button_up_event(ctx, mb, x, y);
    }

    fn char_event(
        &mut self,
        _ctx: &mut mq::Context,
        character: char,
        _keymods: mq::KeyMods,
        _repeat: bool,
    ) {
        self.egui_mq.char_event(character);
    }

    fn key_down_event(
        &mut self,
        ctx: &mut mq::Context,
        keycode: mq::KeyCode,
        keymods: mq::KeyMods,
        _repeat: bool,
    ) {
        // hacky way to work around weird mouse lock in native
        if keycode == mq::KeyCode::Escape {
            self.current_capture = None;

            // brih
            ctx.set_cursor_grab(true);
            ctx.set_cursor_grab(false);

            ctx.show_mouse(true);
        } else if let Some(current_capture) = &self.current_capture {
            // input handling code for map view here...
        }

        self.egui_mq.key_down_event(ctx, keycode, keymods);
    }

    fn key_up_event(&mut self, _ctx: &mut mq::Context, keycode: mq::KeyCode, keymods: mq::KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }
}

fn main() {
    let conf = mq::conf::Conf {
        high_dpi: true,
        ..Default::default()
    };
    mq::start(conf, |mut ctx| {
        mq::UserData::owning(Stage::new(&mut ctx), ctx)
    });
}
