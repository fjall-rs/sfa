// Copyright (c) 2025-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use clap::{Parser, Subcommand};
use globset::Glob;
use pretty_hex::{HexConfig, PrettyHex};
use sfa::{Reader, Writer};
use std::fs::File;
use std::io::{Read, Write};

mod tool_sanitize;

use tool_sanitize::sanitize_path;

#[macro_export]
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
        content_dump: bool,

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
            content_dump,
            section,
        } => {
            dump_command(&file, content_dump, section.as_deref());
        }
        Commands::Create {
            output,
            files,
            force,
        } => {
            create_command(&output, &files, force);
        }
        Commands::Extract {
            file,
            force,
            block_size,
            section,
        } => {
            extract_command(&file, force, section.as_deref(), block_size);
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
                format!("\"{}\"", s)
            } else {
                format!("{:?}", s)
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

fn create_command(output: &std::path::Path, files: &[std::path::PathBuf], force: bool) {
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

    for input_file in files {
        // Read the input file
        let content = match std::fs::read(input_file) {
            Ok(c) => c,
            Err(e) => {
                die!("Error reading file {}: {}", input_file.display(), e);
            }
        };

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

        // Write the file content
        if let Err(e) = writer.write_all(&content) {
            die!("Error writing section {}: {}", section_name, e);
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

fn extract_command(
    file: &std::path::Path,
    force: bool,
    section_pattern: Option<&str>,
    block_size: usize,
) {
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
    for entry in toc.iter() {
        total_count += 1;
        if !section_matches(entry, matcher.as_ref()) {
            continue;
        }

        match_count += 1;
        // Determine the output filename from the section name
        // Sanitize to prevent path traversal attacks
        let output_filename = if entry.name().is_empty() {
            die!("cannot extract section with empty name");
        } else {
            match std::str::from_utf8(entry.name()) {
                Ok(name) => match sanitize_path(name) {
                    Ok(path) => path,
                    Err(e) => {
                        die!("Error: {}", e);
                    }
                },
                Err(_) => {
                    die!(
                        "Error: section name contains invalid UTF-8: {:?}",
                        entry.name()
                    );
                }
            }
        };

        // Check if the file already exists
        if output_filename.exists() && !force {
            die!(
                "Error: file {} already exists. Use --force to overwrite.",
                output_filename.display()
            );
        }

        // Open output file for writing
        let mut output_file = match File::create(&output_filename) {
            Ok(f) => f,
            Err(e) => {
                die!("Error creating file {}: {}", output_filename.display(), e);
            }
        };

        let mut chunk = vec![0u8; block_size];
        match entry.buf_reader(file) {
            Ok(mut reader) => {
                'eof: loop {
                    match reader.read(&mut chunk) {
                        Ok(0) => break 'eof, // EOF
                        Ok(n) => {
                            let data = &chunk[..n];
                            if let Err(e) = output_file.write_all(data) {
                                die!("Error writing to file {}: {}", output_filename.display(), e);
                            }
                        }
                        Err(e) => {
                            die!("Error reading section {}: {}", output_filename.display(), e);
                        }
                    }
                }
            }
            Err(e) => {
                die!("Error opening section {}: {}", output_filename.display(), e);
            }
        }

        // Sync the file to disk
        if let Err(e) = output_file.sync_all() {
            die!("Error syncing file {}: {}", output_filename.display(), e);
        }

        println!("Extracted: {}", output_filename.display());
    }

    if section_pattern.is_some() {
        println!("\nExtracted {match_count} of {total_count} sections.");
    } else {
        println!("\nExtracted {total_count} sections.");
    }
}
