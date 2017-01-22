[![Build Status](https://travis-ci.org/aswaving/yenc.svg?branch=master)](https://travis-ci.org/aswaving/yenc)
# yenc

Encodes bytes into yEnc text and decodes yEnc encoded text back to bytes.
See [documentation](http://docs.rs/yenc).
For more information on yEnc see [Wikipedia](https://en.wikipedia.org/wiki/YEnc) and [yenc.org](http://www.yenc.org).

The public API is not yet stable and will change until v0.1.0.

## Example: encoding Cargo.toml

```
let input_filename = "Cargo.toml";
let mut input_file = std::fs::File::open(&input_filename).unwrap();
let encode_options = yenc::EncodeOptions::new()
    .parts(1)
    .line_length(128);
let mut output_file = std::fs::File::create("Cargo.toml.yenc").unwrap();

yenc::yencode_file(&mut input_file, 
                   &input_filename, 
                   encode_options, 
                   &mut output_file)
    .unwrap();
```

results in a new file Cargo.toml.yenc

```
=ybegin line=128 size=302 name=Cargo.toml 
���������4����JgJL����L4�������JgJLZXZX\L4�������JgJ�L��������Jf�X�������j�����X���hL�4�����������JgJL��������J���J��������J����
�L4����������JgJL�����dYY������X���Y��������Y����L4�������������JgJL�����dYY����X��Y����YZXZX\Y����YL4��������JgJ�L����LVJL�����
�LVJL������L�4�������JgJLws~L44��������������4
=yend size=302 crc32=FB24333B
```

