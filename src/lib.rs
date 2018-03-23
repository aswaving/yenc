//! [yEnc](http://www.yenc.org) is an encoding scheme to include binary files in Usenet messages.
mod crc32;
mod constants;
mod errors;
mod encode;
mod decode;

pub use errors::DecodeError;
pub use encode::{encode_buffer, encode_file, EncodeOptions};
pub use decode::{decode_buffer, decode_file, decode_stream};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality() {
        let b = (0..256).map(|c| c as u8).collect::<Vec<u8>>();
        assert_eq!(
            b,
            {
                let mut col = 0;
                let mut output = Vec::new();
                encode_buffer(&b, &mut col, 128, &mut output);
                decode_buffer(&output)
            }.unwrap()
                .as_slice()
        );
    }
}
