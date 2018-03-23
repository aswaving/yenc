[![Build Status](https://travis-ci.org/aswaving/yenc.svg?branch=master)](https://travis-ci.org/aswaving/yenc)
<<<<<<< Updated upstream
[![Coveralls](https://img.shields.io/coveralls/aswaving/yenc.svg)](https://coveralls.io/github/aswaving/yenc)

=======
[![Rust version]( https://img.shields.io/badge/rust-1.13+-blue.svg)]()
[![Documentation](https://docs.rs/yenc/badge.svg)](https://docs.rs/yenc)
[![Latest version](https://img.shields.io/crates/v/yenc.svg)](https://crates.io/crates/yenc)
[![All downloads](https://img.shields.io/crates/d/yenc.svg)](https://crates.io/crates/yenc)
[![Downloads of latest version](https://img.shields.io/crates/dv/yenc.svg)](https://crates.io/crates/yenc)
>>>>>>> Stashed changes
# yenc

Encodes bytes into yEnc text and decodes yEnc encoded text back to bytes.
Requires compiler version >= v1.17.0.

See [documentation](http://docs.rs/yenc).
For more information on yEnc see [Wikipedia](https://en.wikipedia.org/wiki/YEnc) and [yenc.org](http://www.yenc.org).

The public API is not yet stable.

## Example: encoding Cargo.toml

```
let input_filename = "Cargo.toml";
let mut input_file = std::fs::File::open(&input_filename).unwrap();
let encode_options = yenc::EncodeOptions::new()
    .parts(1)
    .line_length(128);
let mut output_file = std::fs::File::create("Cargo.toml.yenc").unwrap();

yenc::yencode_file(&mut input_file, 
                   input_filename, 
                   encode_options, 
                   &mut output_file)
    .unwrap();
```

results in a new file Cargo.toml.yenc

```
=ybegin line=128 size=302 name=Cargo.toml 
<<<<<<< Updated upstream
<yenc encoded data>
=======
<the encoded bytes>
>>>>>>> Stashed changes
=yend size=302 crc32=FB24333B
```

