// Rust implementation of the cat command.
// Run with: cargo run -- -Asn tests/test.txt

use clap::Parser;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;

// Constant for stdin file name.
const FILENAME_STDIN: &str = "-";

/// Argument parser
#[derive(Parser)]
#[clap(about = "Concatenate FILE(s) to standard output.\n\nWith no FILE, or when FILE is -, read \
                standard input.",
       after_help = "Examples:\n   cat f - g  Output f's contents, then standard input, then g's \
                     contents.\n   cat        Copy standard input to standard output.",
       long_about = None, version)]
struct Args {
    /// equivalent to -vET
    #[arg(long, short = 'A')]
    show_all: bool,
    /// number nonempty output lines, overrides -n
    #[arg(long, short = 'b')]
    number_nonblank: bool,
    /// equivalent to -vE
    #[arg(short = 'e')]
    e: bool,
    /// display $ at end of each line
    #[arg(long, short = 'E')]
    show_ends: bool,
    /// number all output lines
    #[arg(long, short = 'n')]
    number: bool,
    /// suppress repeated empty output lines
    #[arg(long, short = 's')]
    squeeze_blank: bool,
    /// equivalent to -vT
    #[arg(short = 't')]
    t: bool,
    /// display TAB characters as ^I
    #[arg(long, short = 'T')]
    show_tabs: bool,
    /// (ignored)
    #[arg(short = 'u')]
    u: bool,
    /// use ^ and M- notation, except for LFD and TAB
    #[arg(long, short = 'v')]
    show_non_printing: bool,
    // Inpute files (default to stdin if none is provided)
    #[arg(default_values_t = [FILENAME_STDIN.to_string()], hide_default_value = true)]
    files: Vec<String>,
}

// Add formatting to the buffer based on the input arguments.
fn format_buffer(line: &mut Vec<u8>, args: &Args, line_number: &mut i32, newlines: &mut i32) {
    let is_new_line = line.len() == 1 && line[0] == 10;
    let new_line_idx = line.iter().position(|&x| x == 10);

    if is_new_line && args.squeeze_blank {
        *newlines += 1;

        if *newlines > 1 {
            line.clear();
            return;
        }
    } else {
        // Not an empty line.
        *newlines = 0;
    }

    // Show ends.
    if args.show_ends {
        line.insert(new_line_idx.unwrap(), b'$');
    }

    // Show non-printing.
    if args.show_non_printing {
        *line = line
            .iter()
            .flat_map(|c| {
                if *c < 32 && *c != b'\n' && *c != b'\t' {
                    vec![b'^', *c + 64]
                } else if *c == 127 {
                    vec![b'^', b'?']
                } else if *c > 127 {
                    if *c >= 128 + 32 {
                        if *c < 255 {
                            vec![b'M', b'-', *c - 128]
                        } else {
                            vec![b'M', b'-', b'^', b'?']
                        }
                    } else {
                        vec![b'M', b'-', b'^', *c - 128 + 64]
                    }
                } else {
                    vec![*c]
                }
            })
            .collect();
    }

    // Show tabs.
    if args.show_tabs {
        *line = line
            .iter()
            .flat_map(|c| {
                if *c == b'\t' {
                    vec![b'^', b'I']
                } else {
                    vec![*c]
                }
            })
            .collect();
    }

    // Add line numbers.
    if args.number || args.number_nonblank && !is_new_line {
        line.splice(0..0, line_number.to_string().bytes().chain(vec![b' ']));
        *line_number += 1;
    }
}

// Cat: read from input and print to stdout adding formatting if needed.
fn cat(args: &Args, file: &String, needs_formatting: bool, line_number: &mut i32) {
    let mut reader: Box<dyn BufRead> = if file == FILENAME_STDIN {
        // Read from stdin.
        Box::new(BufReader::new(io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(file).unwrap()))
    };
    let mut line: Vec<u8> = Vec::new();
    let mut newlines: i32 = 0;

    // Iterate over the reader line by line.
    loop {
        match reader.read_until(b'\n', &mut line) {
            Ok(bytes_read) if bytes_read > 0 => {
                if !needs_formatting {
                    // Print the buffer to stdout as is.
                    io::stdout().write_all(line.as_slice()).unwrap();
                } else {
                    format_buffer(&mut line, args, line_number, &mut newlines);
                    io::stdout().write_all(line.as_slice()).unwrap();
                }

                line.clear();
            }
            Ok(_) => break, // EOF.
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                break;
            }
        }
    }
}

fn main() {
    let mut args = Args::parse();

    // Set aliases and overrides.
    if args.e {
        args.show_ends = true;
        args.show_non_printing = true;
    }

    if args.number_nonblank {
        args.number = false;
    }

    if args.show_all {
        args.show_ends = true;
        args.show_non_printing = true;
        args.show_tabs = true;
    }

    if args.t {
        args.show_non_printing = true;
        args.show_tabs = true;
    }

    // Check if the input needs to be manipulated before printing.
    let needs_formatting = args.number
        || args.number_nonblank
        || args.show_ends
        || args.squeeze_blank
        || args.show_tabs
        || args.show_non_printing;

    // Line number, increases across files.
    let mut line_number: i32 = 1;

    for file in &args.files {
        cat(&args, file, needs_formatting, &mut line_number);
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::prelude::*;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::Command;
    use std::process::Stdio;

    // Test cat of a single file without formatting.
    #[test]
    fn test_cat_no_formatting() {
        let mut cmd = Command::cargo_bin("cat").unwrap();
        let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_path.push("tests/test.txt");
        let expected_output: Vec<u8> = vec![
            116, 101, 115, 116, 9, 9, 10, 10, 10, 10, 9, 9, 116, 101, 115, 116, 10, 116, 101, 115,
            116, 10, 0, 1, 2, 3, 10, 127, 10, 128, 129, 10, 160, 161, 10, 255, 10,
        ];

        let output = cmd
            .arg(test_path.into_os_string().into_string().unwrap())
            .output()
            .unwrap();
        assert_eq!(output.stdout, expected_output);
    }

    // Test cat of a single file with different formatting options.
    #[test]
    fn test_cat_with_formatting() {
        let mut cmd = Command::cargo_bin("cat").unwrap();
        let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_path.push("tests/test.txt");
        let test_string = test_path.into_os_string().into_string().unwrap();
        let mut expected_output: Vec<u8> = vec![
            49, 32, 116, 101, 115, 116, 94, 73, 94, 73, 36, 10, 50, 32, 36, 10, 51, 32, 94, 73, 94,
            73, 116, 101, 115, 116, 36, 10, 52, 32, 116, 101, 115, 116, 36, 10, 53, 32, 94, 64, 94,
            65, 94, 66, 94, 67, 36, 10, 54, 32, 94, 63, 36, 10, 55, 32, 77, 45, 94, 64, 77, 45, 94,
            65, 36, 10, 56, 32, 77, 45, 32, 77, 45, 33, 36, 10, 57, 32, 77, 45, 94, 63, 36, 10,
        ];

        // Show all, squeeze blanks and show all numbers.
        let mut output = cmd.arg("-Asn").arg(test_string.clone()).output().unwrap();
        assert_eq!(output.stdout, expected_output);

        // Verify -b option overrides -n.
        cmd = Command::cargo_bin("cat").unwrap();
        expected_output = vec![
            49, 32, 116, 101, 115, 116, 94, 73, 94, 73, 36, 10, 36, 10, 50, 32, 94, 73, 94, 73,
            116, 101, 115, 116, 36, 10, 51, 32, 116, 101, 115, 116, 36, 10, 52, 32, 94, 64, 94, 65,
            94, 66, 94, 67, 36, 10, 53, 32, 94, 63, 36, 10, 54, 32, 77, 45, 94, 64, 77, 45, 94, 65,
            36, 10, 55, 32, 77, 45, 32, 77, 45, 33, 36, 10, 56, 32, 77, 45, 94, 63, 36, 10,
        ];
        output = cmd.arg("-Asnb").arg(test_string.clone()).output().unwrap();
        assert_eq!(output.stdout, expected_output);
    }

    // Test cat of multiple files and stdin.
    #[test]
    fn test_cat_multiple() {
        let mut cmd = Command::cargo_bin("cat").unwrap();
        let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_path.push("tests/test.txt");
        let test_string = test_path.into_os_string().into_string().unwrap();
        let expected_output: Vec<u8> = vec![
            49, 32, 116, 101, 115, 116, 94, 73, 94, 73, 36, 10, 50, 32, 36, 10, 51, 32, 94, 73, 94,
            73, 116, 101, 115, 116, 36, 10, 52, 32, 116, 101, 115, 116, 36, 10, 53, 32, 94, 64, 94,
            65, 94, 66, 94, 67, 36, 10, 54, 32, 94, 63, 36, 10, 55, 32, 77, 45, 94, 64, 77, 45, 94,
            65, 36, 10, 56, 32, 77, 45, 32, 77, 45, 33, 36, 10, 57, 32, 77, 45, 94, 63, 36, 10, 49,
            48, 32, 116, 101, 115, 116, 36, 10, 49, 49, 32, 116, 101, 115, 116, 94, 73, 94, 73, 36,
            10, 49, 50, 32, 36, 10, 49, 51, 32, 94, 73, 94, 73, 116, 101, 115, 116, 36, 10, 49, 52,
            32, 116, 101, 115, 116, 36, 10, 49, 53, 32, 94, 64, 94, 65, 94, 66, 94, 67, 36, 10, 49,
            54, 32, 94, 63, 36, 10, 49, 55, 32, 77, 45, 94, 64, 77, 45, 94, 65, 36, 10, 49, 56, 32,
            77, 45, 32, 77, 45, 33, 36, 10, 49, 57, 32, 77, 45, 94, 63, 36, 10,
        ];

        // File, stdin, file.
        let mut child = cmd
            .arg("-Asn")
            .arg(test_string.clone())
            .arg("-")
            .arg(test_string.clone())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        // Collect stdin, send a message and drop to close.
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"test\n").unwrap();

        let output = child.wait_with_output().unwrap();
        assert_eq!(output.stdout, expected_output);
    }
}
