<p align="center">
  <a href="https://github.com/fjall-rs/sfa/actions/workflows/test.yml">
      <img src="https://github.com/fjall-rs/sfa/actions/workflows/test.yml/badge.svg" alt="CI" />
  </a>
  <a href="https://docs.rs/sfa">
    <img src="https://img.shields.io/docsrs/sfa?color=green" alt="docs.rs" />
  </a>
  <a href="https://crates.io/crates/sfa">
    <img src="https://img.shields.io/crates/v/sfa?color=blue" alt="Crates.io" />
  </a>
  <img src="https://img.shields.io/badge/MSRV-1.89.0-blue" alt="MSRV" />
  <a href="https://deps.rs/repo/github/fjall-rs/sfa">
    <img src="https://deps.rs/repo/github/fjall-rs/sfa/status.svg" alt="dependency status" />
  </a>
</p>

*SFA* (**s**imple **f**ile **a**rchive) is a minimal, flat file archive encoding/decoding library for Rust.

The file can be segmented into multiple sections (similar to a zip file), and individual sections accessed as a `std::io::Read` or `(offset, len)` tuples.

## Sponsors

<a href="https://sqlsync.dev">
  <picture>
    <source width="240" alt="Orbitinghail" media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/fjall-rs/fjall-rs.github.io/d22fcb1e6966ce08327ea3bf6cf2ea86a840b071/public/logos/orbitinghail.svg" />
    <source width="240" alt="Orbitinghail" media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/fjall-rs/fjall-rs.github.io/d22fcb1e6966ce08327ea3bf6cf2ea86a840b071/public/logos/orbitinghail_dark.svg" />
    <img width="240" alt="Orbitinghail" src="https://raw.githubusercontent.com/fjall-rs/fjall-rs.github.io/d22fcb1e6966ce08327ea3bf6cf2ea86a840b071/public/logos/orbitinghail_dark.svg" />
  </picture>
</a>

## Basic usage

```bash
cargo add sfa
```

```rust
use sfa::{Writer, Reader};
use std::{
  fs::File,
  io::{BufWriter, Read, Write}
};

let file = File::create(&path)?;
let mut file = BufWriter::new(file);
let mut writer = Writer::from_writer(&mut file);

writer.start("Section 1")?;
writer.write_all(b"Hello world!\n")?;

writer.finish()?;
file.get_mut().sync_all()?;
drop(file);
// If on Unix, you probably want to fsync the directory here

let reader = Reader::new(&path)?;
let toc = reader.toc();
assert_eq!(toc.len(), 1);
assert_eq!(toc[0].name(), b"Section 1");
assert_eq!(toc[0].len(), 13);

let reader = toc[0].buf_reader(&path).unwrap();
assert_eq!(b"Hello world!\n", &*reader.bytes().collect::<Result<Vec<_>, _>>()?);
```

## Stable disk format

The disk format will be stable as of 1.0.0.

Future breaking changes will result in a major version bump.

```ini
??? (header content)
[section1]
  ??? (section1 content)
[section2]
  ??? (section2 content)
[toc]
[magic, 4 bytes]
[len, 4 bytes]
  <section pos, 8 bytes>
  <section len, 8 bytes>
  <section name, len = N, 2 bytes>
  <section name, N bytes>
...
[trailer]
[magic, 4 bytes]
[version, 1 byte, 0x1]
[checksum type, 1 byte, always 0x0]
[toc checksum, 16 bytes]
[toc pos, 8 bytes]
[toc len, 8 bytes]
```

All integers are little-endian encoded.

## License

All source code is licensed under MIT OR Apache-2.0.

All contributions are to be licensed as MIT OR Apache-2.0.
