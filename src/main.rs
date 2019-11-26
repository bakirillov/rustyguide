extern crate regex;
extern crate serde_json;
#[macro_use]
extern crate lazy_static;

use std::fs;
use std::str;
use regex::Regex;
use docopt::Docopt;
use std::borrow::Cow;
use serde::{Deserialize};
use seq_io::fasta::{Reader};
use serde_json::{Result, Value};
use indicatif::{ProgressBar,ProgressStyle};

use std::fs::OpenOptions;
use std::io::prelude::*;



const USAGE: &'static str = "
rustyrgram.

Usage:
  rustyguide scan <pam> <before> <length> <input> <output>
  rustyguide test <pam>
  rustyrgram (-h | --help)
  rustyrgram --version

Options:
  -h --help     Show this screen.
  --version     Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    cmd_scan: bool,
    arg_pam: String,
    arg_before: String,
    arg_length: String,
    arg_input: String,
    arg_output: String
}

fn replace_ns(before: &str) -> Cow<str> {
    lazy_static! {
        static ref UNKNOWN_REGEX : Regex = Regex::new(r"N+").unwrap();
    }
    UNKNOWN_REGEX.replace_all(before, " ")
}

fn match_pam(before: &str, pam: &str) -> bool {
    let re = Regex::new(&format!(r"{}", pam)).unwrap();
    re.is_match(before)
}

fn main() {
    println!("{}", "Running on-target search");
    let args = Docopt::new(USAGE)
                      .and_then(|dopt| dopt.parse())
                      .unwrap_or_else(|e| e.exit());
    if args.get_bool("test") {
        let PAM = args.get_str("<pam>");
        let should_be_false = match_pam("CCC", &PAM);
        let should_be_true = match_pam("CGG", &PAM);
        println!("should_be_false, {}", should_be_false);
        println!("should_be_true, {}", should_be_true);
    }
    if args.get_bool("scan") {
        let PAM = args.get_str("<pam>");
        let PAM_regex = PAM.replace("N", "[ATGC]");
        let before = args.get_str("<before>").to_string().parse().unwrap();
        let length = args.get_str("<length>").parse::<usize>().unwrap();
        let input_fn = args.get_str("<input>").to_string();
        let output_fn = args.get_str("<output>").to_string();
        let spinner = ProgressBar::new_spinner();
        let mut message = "File ".to_string();
        message.push_str(&input_fn);
        message.push_str(" - elapsed [{elapsed}] : [{eta}] left {spinner}");
        spinner.set_style(ProgressStyle::default_bar().template(&message));
        let mut reader = Reader::from_path(input_fn).unwrap();
        let mut counter = 0;
        while let Some(result) = reader.next() {
            spinner.tick();
            let record = result.unwrap();
            let mut full_text = record.full_seq();
            let full_text = full_text.to_mut();
            let full_text = str::from_utf8(&full_text).unwrap().to_uppercase();
            let full_text = replace_ns(&full_text);
            let chunks:Vec<&str> = full_text.split(" ").collect();
            let bar = ProgressBar::new(chunks.len() as u64);
            bar.set_style(
                ProgressStyle::default_bar().template(
                    "Chunk #{pos} - elapsed [{elapsed}] : [{eta}] left, {bar:100} {len}"
                )
            );
            let mut file = OpenOptions::new().append(
                true
            ).create(true).open(&output_fn).unwrap();
            for i in 0..chunks.len() {
                let text = chunks[i];
                let inter = text.chars().collect::<Vec<char>>();
                let mut windows = inter.windows(length as usize);
                let bar2 = ProgressBar::new(windows.len() as u64);
                bar2.set_style(
                    ProgressStyle::default_bar().template(
                        "Window #{pos} - elapsed [{elapsed}] : [{eta}] left, {bar:100} {len}"
                    )
                );
                for a in windows {
                    let mut p_guide = a.iter().cloned().collect::<String>();
                    let p_pam: String;
                    if before {
                        p_pam = p_guide[0..PAM.len()].to_string();
                    } else {
                        p_pam = p_guide[(length-PAM.len() as usize)..length].to_string();
                    }
                    if match_pam(&p_pam, &PAM_regex) {
                        write!(&mut file, "{}\n", p_guide);
                    }
                    bar2.inc(1);
                }
                bar.inc(1);
            }
            bar.finish_and_clear();
            counter = counter + 1;
        }
        spinner.finish();
    }
}
