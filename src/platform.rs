#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

pub fn file_picker_map() -> Option<(String, Vec<u8>)> {
    file_picker("Map file", &["bsp"])
}

pub fn file_picker_json() -> Option<(String, Vec<u8>)> {
    file_picker("JSON", &["json", "txt"])
}

pub fn save_picker_zip(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    Ok(save_picker("zip", &["zip"], data)?)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn file_picker(filter_name: &str, extensions: &[&str]) -> Option<(String, Vec<u8>)> {
    use std::fs;

    let file = rfd::FileDialog::new()
        .add_filter(filter_name, extensions)
        .pick_file();

    if let Some(file) = file {
        let file_stem = file.file_stem().unwrap().to_str().unwrap().to_string();
        if let Ok(vec) = fs::read(file) {
            Some((file_stem, vec))
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_picker(
    filter_name: &str,
    extensions: &[&str],
    data: &[u8],
) -> Result<(), std::io::Error> {
    use std::fs;

    let file = rfd::FileDialog::new()
        .add_filter(filter_name, extensions)
        .save_file();

    if let Some(file) = file {
        fs::write(file, data)
    } else {
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
pub fn save_picker(
    filter_name: &str,
    extensions: &[&str],
    data: &[u8],
) -> Result<(), std::io::Error> {
    todo!()
}

#[cfg(target_arch = "wasm32")]
pub fn file_picker(filter_name: &str, extensions: &[&str]) -> Option<(String, Vec<u8>)> {
    todo!()
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    pub fn js_file_picker(map_vec: u32, blacklist: u32, ctx: u32);

    pub fn js_save_file(name: u32, name_sz: u32, data: u32, data_sz: u32);
}

#[cfg(target_arch = "wasm32")]
pub fn wasm_file_picker(
    map_vec: *mut Vec<Rc<RefCell<crate::map_window::MapWindowStage>>>,
    blacklist: *const Option<crate::blacklist::Blacklist>,
    ctx: *mut miniquad::Context,
) {
    use std::ffi::CString;

    unsafe {
        console_log(
            CString::new(format!("{}", map_vec as u32))
                .unwrap()
                .as_ptr(),
        );
    };
    unsafe { js_file_picker(map_vec as u32, blacklist as u32, ctx as u32) };
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
// wasm-bindgen
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    use std::alloc::{alloc, Layout};
    use std::mem;

    let align = mem::align_of::<usize>();
    if let Ok(layout) = Layout::from_size_align(size, align) {
        unsafe {
            if layout.size() > 0 {
                let ptr = alloc(layout);
                if !ptr.is_null() {
                    return ptr;
                }
            } else {
                return align as *mut u8;
            }
        }
    }

    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
// Sadly was given away cuz compiling step thinks this is the C free
// pub unsafe extern "C" fn free(ptr: *mut u8, size: usize) {
pub unsafe extern "C" fn free(ptr: *mut u8) {
    use std::alloc::{dealloc, Layout};
    use std::mem;

    // This happens for zero-length slices, and in that case `ptr` is
    // likely bogus so don't actually send this to the system allocator
    // if size == 0 {
    //     return;
    // }
    // let align = mem::align_of::<usize>();
    // let layout = Layout::from_size_align_unchecked(size, align);
    // dealloc(ptr, layout);
}

// fucking async JS
#[cfg(target_arch = "wasm32")]
#[no_mangle]
// pub fn wasm_cb(map_vec: &mut Vec<Rc<RefCell<crate::map_window::MapWindowStage>>>, blacklist: Option<&crate::blacklist::Blacklist>, ctx: &mut miniquad::Context, stem: String, vec: Vec<u8>) {
// pub extern "C" fn wasm_cb(map_vec: u32, blacklist: u32, ctx: u32, stem: String, vec: Vec<u8>) {
pub extern "C" fn wasm_cb(
    map_vec: u32,
    blacklist: u32,
    ctx: u32,
    stem_ptr: u32,
    stem_len: u32,
    vec_ptr: u32,
    vec_len: u32,
) {
    use crate::map_window::MapWindowStage;
    use std::ffi::CString;

    let map_vec = unsafe {
        (map_vec as *mut Vec<Rc<RefCell<crate::map_window::MapWindowStage>>>)
            .as_mut()
            .unwrap()
    };
    let blacklist = unsafe { &*(blacklist as *const Option<crate::blacklist::Blacklist>) };
    let ctx = unsafe { &mut *(ctx as *mut miniquad::Context) };

    let stem_slice =
        unsafe { std::slice::from_raw_parts(stem_ptr as *const u8, stem_len as usize) };
    let stem = unsafe { std::str::from_utf8_unchecked(stem_slice) }.to_string();

    let vec_slice = unsafe { std::slice::from_raw_parts(vec_ptr as *const u8, vec_len as usize) };
    let vec = vec_slice.to_vec();

    match MapWindowStage::new(stem, vec, ctx, 256, 256, blacklist.as_ref()) {
        Ok(v) => {
            unsafe {
                console_log(CString::new(format!("OK!")).unwrap().as_ptr());
            };
            map_vec.push(Rc::new(RefCell::new(v)));
            unsafe {
                console_log(CString::new(format!("PUSHED!")).unwrap().as_ptr());
            };
        }
        Err(v) => {
            unsafe {
                console_log(CString::new(format!("Err: {:#?}", v)).unwrap().as_ptr());
            };
        }
    }
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    pub fn console_log(msg: *const ::std::os::raw::c_char);
}
