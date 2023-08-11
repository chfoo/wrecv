mod escape;

pub use escape::*;

pub fn preview_bytes(data: &[u8], length: usize) -> String {
    if data.len() <= length {
        String::from_utf8_lossy(data).into_owned()
    } else {
        let mut text = String::from_utf8_lossy(&data[0..length]).into_owned();
        text.push('…');
        text
    }
}
