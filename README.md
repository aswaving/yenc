# yenc

Encodes bytes into yEnc text and decodes yEnc encoded text back to bytes.
See [documentation](http://docs.rs/yenc).
For more information on yEnc see [Wikipedia](https://en.wikipedia.org/wiki/YEnc) and [yenc.org](http://www.yenc.org).

## Example: encoding Cargo.toml

```
yenc::yencode_file("Cargo.toml", "Cargo.toml.yenc");
```

results in a new file Cargo.toml.yenc

```
=ybegin line=128 size=302 name=Cargo.toml 
���������4����JgJL����L4�������JgJLZXZX\L4�������JgJ�L��������Jf�X�������j�����X���hL�4�����������JgJL��������J���J��������J����
�L4����������JgJL�����dYY������X���Y��������Y����L4�������������JgJL�����dYY����X��Y����YZXZX\Y����YL4��������JgJ�L����LVJL�����
�LVJL������L�4�������JgJLws~L44��������������4
=yend size=302 crc32=FB24333B
```

