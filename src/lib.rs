//! [yEnc](http://www.yenc.org) is an encoding scheme to include binary files in Usenet messages.
//! 
//! The `EncodeOptions` and `DecodeOptions` structs are the entry points for encoding and decoding.
//! 
//! To encode a complete file to a single encoded 
//! ```rust,no_run
//! # extern crate yenc;
//! let encode_options = yenc::EncodeOptions::new();
//! let mut output_file = std::fs::File::create("test.bin.001").unwrap();
//! encode_options.encode_file("test1.bin", &mut output_file).unwrap();
//! ```
//!
//! To decode from a stream and place the targets files in the output directory
//! 
//! ```rust,no_run
//! # extern crate yenc;
//! # use std::io::{Read};
//! let tmpdir = "tmp";
//! std::fs::create_dir(tmpdir).unwrap();
//! let decode_options = yenc::DecodeOptions::new(tmpdir);
//! let message = Vec::<u8>::new();
//! // ...
//! // obtain message from a socket, for example the body of a usenet article.
//! // alternatively, directly read from the NNTP TCPStream.
//! // ...
//! decode_options.decode_stream(message.as_slice()).unwrap();
//! ```
//! 
#![forbid(unsafe_code, missing_docs, missing_debug_implementations)]
mod constants;
mod crc32;
mod decode;
mod encode;
mod errors;

pub use self::decode::{decode_buffer, DecodeOptions};
pub use self::encode::{encode_buffer, EncodeOptions};
pub use self::errors::{DecodeError, EncodeError};

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
