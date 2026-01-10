// Copyright (c) 2025-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use clap::{Parser, Subcommand};
use globset::Glob;
use path_jail;
use pretty_hex::{HexConfig, PrettyHex};
use sfa::{Reader, Writer};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

macro_rules! die {
    ($fmt:literal, $($arg:tt)*) => {{
        eprintln!($fmt, $($arg)*);
        std::process::exit(1);
    }};

    ($msg:literal) => {{
        eprintln!($msg);
        std::process::exit(1);
    }};

    () => {{
        eprintln!("Program terminated unexpectedly");
        std::process::exit(1);
    }};
}

fn parse_block_size(s: &str) -> Result<usize, String> {
    // Use powers-of-two for block size
    let cfg = parse_size::Config::new().with_binary();
    cfg.parse_size(s)
        .map(|size| size as usize)
        .map_err(|e| e.to_string())
}

/// Simple Flat Archive (SFA) command-line tool
///
/// Usage examples (using shorthands):
///
///     Create an SFA file:           sfa c file.sfa f1.txt f2.txt
///     Test and dump an SFA file:    sfa t file.sfa
///     Extract sections as files:    sfa x file.sfa
#[derive(Parser)]
#[command(name = "sfa")]
#[command(arg_required_else_help = true)]
#[command(verbatim_doc_comment)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Dump SFA file sections and metadata
    #[command(visible_aliases = ["d", "test", "t"])]
    #[command(arg_required_else_help = true)]
    Dump {
        /// Path to the SFA file
        file: std::path::PathBuf,

        /// Dump section content in hexdump format
        #[arg(long, short = 'C')]
        content: bool,

        /// Only operate on sections matching the glob pattern
        #[arg(long, short = 's')]
        section: Option<String>,
    },
    /// Create a new SFA file from input files
    #[command(visible_aliases = ["c"])]
    #[command(arg_required_else_help = true)]
    Create {
        /// Path to the SFA file to create
        output: std::path::PathBuf,

        /// Input files to add to the archive
        #[arg(required = true)]
        files: Vec<std::path::PathBuf>,

        /// Overwrite existing file
        #[arg(long, short = 'f')]
        force: bool,

        /// Block size for section content import
        #[arg(
            short = 'b',
            long = "block-size",
            default_value = "64KB",
            value_parser = parse_block_size
        )]
        block_size: usize,
    },
    /// Extract all sections from an SFA file
    #[command(visible_aliases = ["x"])]
    #[command(arg_required_else_help = true)]
    Extract {
        /// Path to the SFA file
        file: std::path::PathBuf,

        /// Overwrite existing files
        #[arg(long, short = 'f')]
        force: bool,

        /// Block size for section content extraction
        #[arg(
            short = 'b',
            long = "block-size",
            default_value = "64KB",
            value_parser = parse_block_size
        )]
        block_size: usize,

        /// Only operate on sections matching the glob pattern
        #[arg(long, short = 's')]
        section: Option<String>,

        /// The destination base path to extract to
        #[arg(short = 'D', long = "dest", default_value = ".")]
        dest: PathBuf,
    },
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    // "args stuffing" to support tar-like command shorthands
    // Only transform if it's a single char command or tar-like shorthand (e.g., "cf", "xf")
    if args.len() >= 2 {
        let arg = args[1].clone();
        if let Some(cmd @ ('d' | 't' | 'x' | 'c')) = arg.chars().next() {
            // Only transform if it's a single character or tar-like shorthand (2-3 chars with flags)
            // Don't transform full command names like "create", "dump", "extract"
            if arg.len() == 1 || (arg.len() <= 3 && arg.chars().skip(1).all(|c| c.is_alphabetic()))
            {
                args.remove(1);
                if arg.len() > 1 {
                    args.insert(1, format!("-{}", &arg[1..]));
                }
                args.insert(1, format!("{}", cmd));
            }
        }
    }

    let args = Args::parse_from(args);

    match args.command {
        Commands::Dump {
            file,
            content,
            section,
        } => {
            dump_command(&file, content, section.as_deref());
        }
        Commands::Create {
            output,
            files,
            force,
            block_size,
        } => {
            create_command(&output, &files, force, block_size);
        }
        Commands::Extract {
            file,
            force,
            block_size,
            section,
            dest,
        } => {
            extract_command(&file, force, section.as_deref(), block_size, &dest);
        }
    }
}

fn build_section_matcher(section_pattern: Option<&str>) -> Option<globset::GlobMatcher> {
    section_pattern.and_then(|pattern| match Glob::new(pattern) {
        Ok(glob) => Some(glob.compile_matcher()),
        Err(e) => {
            die!("Error parsing glob pattern: {}", e);
        }
    })
}

fn section_matches(entry: &sfa::TocEntry, matcher: Option<&globset::GlobMatcher>) -> bool {
    if let Some(matcher) = matcher {
        match std::str::from_utf8(entry.name()) {
            Ok(name) => matcher.is_match(name),
            Err(_) => false,
        }
    } else {
        true
    }
}

fn format_section_name(name: &[u8]) -> String {
    if name.is_empty() {
        return "(empty)".to_string();
    }
    match std::str::from_utf8(name) {
        Ok(s) => {
            if s.chars().all(|c| {
                c.is_ascii()
                    && (c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation())
            }) {
                format!("\"{s}\"")
            } else {
                format!("{s}")
            }
        }
        Err(_) => format!("{:?}", name),
    }
}

fn dump_command(file: &std::path::Path, content_dump: bool, section_pattern: Option<&str>) {
    let reader = match Reader::new(file) {
        Ok(r) => r,
        Err(e) => {
            die!("Error opening SFA file: {}", e);
        }
    };

    let toc = reader.toc();

    if toc.is_empty() {
        println!("SFA file contains no sections.");
        return;
    }

    let matcher = build_section_matcher(section_pattern);

    println!("SFA file: {}", file.display());
    println!("Number of sections: {}\n", toc.len());

    // Process matching sections as we encounter them
    let mut total_count = 0;
    let mut match_count = 0;
    for (original_idx, entry) in toc.iter().enumerate() {
        total_count += 1;
        if !section_matches(entry, matcher.as_ref()) {
            continue;
        }

        if match_count > 0 {
            println!();
        }
        match_count += 1;

        println!("Section {}:", original_idx);
        println!("  Name: {}", format_section_name(entry.name()));
        println!("  Position: {} (0x{:x})", entry.pos(), entry.pos());
        println!("  Length: {} bytes (0x{:x})", entry.len(), entry.len());

        if content_dump {
            println!("  Content:");
            println!();
            match entry.buf_reader(file) {
                Ok(mut reader) => {
                    const BLOCK_SIZE: usize = 4096;
                    let mut buffer = vec![0u8; BLOCK_SIZE];
                    let mut offset = 0u64;

                    loop {
                        match reader.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                let data = &buffer[..n];
                                let cfg = HexConfig {
                                    title: false,
                                    width: 16,
                                    group: 8,
                                    display_offset: offset as usize,
                                    ..HexConfig::default()
                                };
                                println!("{:?}", data.hex_conf(cfg));
                                offset += n as u64;
                            }
                            Err(e) => {
                                eprintln!("Error reading section content: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error opening section: {}", e);
                }
            }
        }
    }

    if section_pattern.is_some() {
        println!("\nDumped {match_count} of {total_count} sections.");
    } else {
        println!("\nDumped {total_count} sections.");
    }
}

fn create_command(
    output: &std::path::Path,
    files: &[std::path::PathBuf],
    force: bool,
    block_size: usize,
) {
    // Check if the output file already exists
    if output.exists() && !force {
        die!(
            "file {} already exists. Use --force to overwrite.",
            output.display()
        );
    }

    let mut file = match File::create(output) {
        Ok(f) => f,
        Err(e) => {
            die!("Error creating SFA file: {}", e);
        }
    };

    let mut writer = Writer::from_writer(&mut file);
    let mut chunk = vec![0u8; block_size];

    for input_file in files {
        // Use the filename (without path) as the section name
        let section_name = input_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| {
                die!("invalid filename for {}", input_file.display());
            });

        // Start a new section
        if let Err(e) = writer.start(section_name) {
            die!("Error starting section {}: {}", section_name, e);
        }

        // Open the input file for reading
        let mut input = match File::open(input_file) {
            Ok(f) => f,
            Err(e) => {
                die!("Error reading file {}: {}", input_file.display(), e);
            }
        };

        // Stream the file content in chunks
        loop {
            match input.read(&mut chunk) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let data = &chunk[..n];
                    if let Err(e) = writer.write_all(data) {
                        die!("Error writing section {}: {}", section_name, e);
                    }
                }
                Err(e) => {
                    die!("Error reading file {}: {}", input_file.display(), e);
                }
            }
        }
    }

    // Finish the archive
    if let Err(e) = writer.finish() {
        die!("Error finishing SFA file: {}", e);
    }

    // Sync the file to disk
    if let Err(e) = file.sync_all() {
        die!("Error syncing SFA file: {}", e);
    }

    println!(
        "Created SFA file: {} with {} sections",
        output.display(),
        files.len()
    );
}

#[cfg(unix)]
fn safely_open_file_or_die(
    dest: &path_jail::Jail,
    output_path_raw: &Path,
    output_path_jailed: &Path,
    force: bool,
) -> File {
    // On Unix, directly open file from raw filename within the dest jail
    // to avoid TOCTOU (Time-of-Check to Time-of-Use) attacks.

    match if force {
        dest.create_or_truncate(output_path_raw)
    } else {
        dest.create(output_path_raw)
    } {
        Ok(f) => f.into_inner(),
        Err(e) => {
            if let path_jail::JailError::Io(io_err) = &e {
                if io_err.kind() == std::io::ErrorKind::AlreadyExists {
                    die!(
                        "File {} already exists. Use --force to overwrite.",
                        output_path_jailed.display()
                    );
                }
            }
            die!("Error opening file {}: {e}", output_path_jailed.display());
        }
    }
}

//#[allow(dead_code)] // Comment in if testing on Unix
#[cfg(not(unix))] // Comment out if testing on Unix
fn safely_open_file_or_die(
    _dest: &path_jail::Jail,
    _output_path_raw: &Path,
    output_path_jailed: &Path,
    force: bool,
) -> File {
    // As the path_jail crate only supports the secure-open feature on unix,
    // we need to handle the other platforms separately. The workaround can
    // be removed once the path_jail crate supports the secure-open feature
    // on all platforms.

    let mut options = std::fs::OpenOptions::new();
    options.write(true);

    if force {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }

    match options.open(output_path_jailed) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            die!(
                "File {} already exists. Use --force to overwrite.",
                output_path_jailed.display()
            )
        }
        Err(e) => {
            die!("Error creating file {}: {e}", output_path_jailed.display());
        }
    }
}

fn extract_command(
    file: &std::path::Path,
    force: bool,
    section_pattern: Option<&str>,
    block_size: usize,
    dest: &std::path::Path,
) {
    let dest = match path_jail::Jail::new(dest) {
        Ok(p) => p,
        Err(e) => {
            die!(
                "Error creating path jail for destination {}: {e}",
                dest.display()
            );
        }
    };

    let reader = match Reader::new(file) {
        Ok(r) => r,
        Err(e) => {
            die!("Error opening SFA file: {}", e);
        }
    };

    let toc = reader.toc();

    if toc.is_empty() {
        println!("SFA file contains no sections.");
        return;
    }

    let matcher = build_section_matcher(section_pattern);

    // Process matching sections, counting as we go
    let mut total_count = 0;
    let mut match_count = 0;
    let mut chunk = vec![0u8; block_size];

    for entry in toc.iter() {
        total_count += 1;
        if !section_matches(entry, matcher.as_ref()) {
            continue;
        }

        match_count += 1;

        let section_name = match std::str::from_utf8(entry.name()) {
            Ok(s) => s,
            Err(_) => {
                die!(
                    "Error: section name contains invalid UTF-8: {:?}",
                    entry.name()
                );
            }
        };
        let output_path_raw = Path::new(section_name);
        let output_path_jailed = match dest.join(output_path_raw) {
            Ok(p) => p,
            Err(e) => {
                die!(
                    "Error jailing path {} to dest {}: {e}",
                    output_path_raw.display(),
                    dest.root().display()
                );
            }
        };

        let mut output_file_jailed =
            safely_open_file_or_die(&dest, &output_path_raw, &output_path_jailed, force);

        match entry.buf_reader(file) {
            Ok(mut reader) => {
                'eof: loop {
                    match reader.read(&mut chunk) {
                        Ok(0) => break 'eof, // EOF
                        Ok(n) => {
                            let data = &chunk[..n];
                            if let Err(e) = output_file_jailed.write_all(data) {
                                die!(
                                    "Error writing to file {}: {e}",
                                    output_path_jailed.display()
                                );
                            }
                        }
                        Err(e) => {
                            die!(
                                "Error reading section {}: {e}",
                                output_path_jailed.display()
                            );
                        }
                    }
                }
            }
            Err(e) => {
                die!("Error opening section {section_name}: {e}");
            }
        }

        // Sync the file to disk
        if let Err(e) = output_file_jailed.sync_all() {
            die!("Error syncing file {}: {e}", output_path_jailed.display());
        }

        println!(
            "Extracted: {section_name} to {}",
            output_path_jailed.display()
        );
    }

    if section_pattern.is_some() {
        println!("\nExtracted {match_count} of {total_count} sections.");
    } else {
        println!("\nExtracted {total_count} sections.");
    }
}
