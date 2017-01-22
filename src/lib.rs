//! [yEnc](http://www.yenc.org) is an encoding scheme to include binary files in Usenet messages.
mod crc32;
mod constants;
mod errors;
mod encode;
mod decode;

pub use errors::DecodeError;
pub use encode::{yencode_file, yencode_buffer, EncodeOptions};
pub use decode::{ydecode_file, ydecode_buffer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality() {
        let b = (0..256).map(|c| c as u8).collect::<Vec<u8>>();
        let mut col = 0;
        assert_eq!(b,
                   ydecode_buffer(&yencode_buffer(&b, &mut col, 128)).unwrap().as_slice());
    }
}
