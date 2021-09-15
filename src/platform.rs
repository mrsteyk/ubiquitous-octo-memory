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
