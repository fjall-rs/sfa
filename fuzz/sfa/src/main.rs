#[macro_use]
extern crate afl;

use arbitrary::{Arbitrary, Unstructured};
use sfa::{Reader, Writer};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};

fn main() {
    fuzz!(|data: &[u8]| {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.sfa");

        let mut u = Unstructured::new(data);

        let num_sections = u.int_in_range::<usize>(1..=5).unwrap_or(1);

        let mut expected = Vec::new();
        let mut seen = HashSet::new();

        {
            let mut file = File::create(&path).unwrap();
            let mut writer = Writer::from_writer(&mut file);

            for i in 0..num_sections {
                let mut name: String =
                    String::arbitrary(&mut u).unwrap_or_else(|_| format!("section_{i}"));

                if name.is_empty() {
                    name = format!("section_{i}");
                }
                while seen.contains(&name) {
                    name.push('x');
                }
                seen.insert(name.clone());

                let mut content: Vec<u8> =
                    Vec::<u8>::arbitrary(&mut u).unwrap_or_else(|_| vec![b'x']);

                if content.is_empty() {
                    content.push(0);
                }

                writer.start(name.as_bytes()).unwrap();
                writer.write_all(&content).unwrap();

                expected.push((name, content));
            }

            writer.finish().unwrap();
        }

        let reader = Reader::new(&path).unwrap();
        let toc = reader.toc();

        // Assert that the number of sections matches.
        assert_eq!(expected.len(), toc.len());

        // Validate each section.
        for (i, (exp_name, exp_content)) in expected.into_iter().enumerate() {
            assert_eq!(exp_name.as_bytes(), toc[i].name());

            let bytes = toc[i]
                .buf_reader(&path)
                .unwrap()
                .bytes()
                .collect::<std::io::Result<Vec<_>>>()
                .unwrap();

            assert_eq!(bytes, exp_content);
        }
    });
}
