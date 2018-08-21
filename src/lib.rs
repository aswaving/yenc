//! [yEnc](http://www.yenc.org) is an encoding scheme to include binary files in Usenet messages.
mod constants;
mod crc32;
mod decode;
mod encode;
mod errors;

pub use decode::{decode_buffer, decode_file, decode_stream};
pub use encode::{encode_buffer, encode_file, encode_stream, EncodeOptions};
pub use errors::DecodeError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality() {
        let b = (0..256).map(|c| c as u8).collect::<Vec<u8>>();
        assert_eq!(
            b,
            {
                let mut output = Vec::new();
                encode_buffer(&b, 0, 128, &mut output).unwrap();
                decode_buffer(&output)
            }.unwrap()
                .as_slice()
        );
    }
}
