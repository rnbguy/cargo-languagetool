//! This main module for cargo grammar checking.
//! Use wisely.

use cargo_grammarly::{Docs, FixedDoc, FixedDocs};

const ENVIRONMENT_VARIABLE_NAME: &str = "GRAMMARLY_API_KEY";

use clap::Parser;
use color_eyre::Result;

#[derive(Parser)]
#[command(name = "cargo-grammarly")]
#[command(version, about)]
struct App {
    #[clap(short, long, value_name = "API_KEY")]
    api_key: Option<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let app = App::parse();

    dotenv::dotenv().ok();

    let api_key = std::env::var(ENVIRONMENT_VARIABLE_NAME)
        .ok()
        .or(app.api_key)
        .expect("Please provide a Grammarly API key");

    let source_directory = get_source_directory();
    check_grammar(&api_key, &fetch_docs(&source_directory));

    Ok(())
}

fn get_source_directory() -> String {
    // TODO make it possible to work with all crates in the workspace
    // Getting the invocation directory.
    format!("{}/src", std::env::var("PWD").unwrap())
}

/// Reads the .rs files in the directory recursively.
fn fetch_docs(dir: &str) -> Vec<Docs> {
    use proc_macro2::TokenStream;

    // dbg!(dir);

    let is_rs = |e: &walkdir::DirEntry| -> bool {
        e.file_type().is_file()
            && std::path::Path::new(e.path().to_str().unwrap())
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("rs"))
    };
    let parse_docs = |path: &String| -> Docs {
        use std::fs;
        let content = fs::read_to_string(path).unwrap();
        let stream: TokenStream = syn::parse_str(&content).unwrap();
        // dbg!(&stream);
        Docs::from((path, stream))
    };

    let files = walkdir::WalkDir::new(dir)
        .max_depth(999)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(is_rs)
        .filter_map(|e| Some(e.path().to_str()?.to_owned()))
        .collect::<Vec<String>>();
    // dbg!(&files);

    files
        .iter()
        .map(parse_docs)
        .filter(|d| !d.0.is_empty())
        .collect()
}

fn doc_checked<'a>(api_key: &str, doc: &'a mut FixedDoc) -> &'a mut FixedDoc {
    doc.check_response = grammarbot_io::Request::from(&doc.text)
        .api_key(api_key)
        .send()
        .ok();
    doc
}

fn docs_checked<'a>(api_key: &str, docs: &'a mut FixedDocs) -> &'a mut FixedDocs {
    for docs in docs.fixed.values_mut() {
        for doc in docs {
            let _ = doc_checked(api_key, doc);
        }
    }
    docs
}

const fn decimal_places(mut num: usize) -> usize {
    let mut places = 1;

    while num % 10 > 1 {
        num /= 10;
        places += 1;
    }

    places
}

fn print_response(file: &str, doc: &FixedDoc) {
    let mut t = term::stdout().unwrap();

    if let Some(grammarbot_io::Response::Success { matches, .. }) = &doc.check_response {
        for m in matches {
            // dbg!(&m);

            let line_width = decimal_places(doc.span.start.line) + 2;

            t.attr(term::Attr::Bold).unwrap();
            t.fg(term::color::RED).unwrap();
            write!(t, "error").unwrap();
            t.fg(term::color::WHITE).unwrap();
            writeln!(t, ": {}", m.short_message).unwrap();
            t.fg(term::color::BLUE).unwrap();
            write!(t, "{:>width$}", "-->", width = line_width).unwrap();
            let _ = t.reset();
            writeln!(t, " {file}:{line}", file = file, line = doc.span.start.line).unwrap();
            t.fg(term::color::BLUE).unwrap();
            t.attr(term::Attr::Bold).unwrap();
            writeln!(t, "{:^width$}| ", " ", width = line_width).unwrap();
            write!(
                t,
                "{line:^width$}| ",
                line = doc.span.start.line,
                width = line_width
            )
            .unwrap();
            let _ = t.reset();
            writeln!(t, "{}", m.sentence).unwrap();
            t.fg(term::color::BLUE).unwrap();
            t.attr(term::Attr::Bold).unwrap();
            write!(t, "{:^width$}| ", " ", width = line_width).unwrap();
            t.fg(term::color::RED).unwrap();
            writeln!(t, "- {}", m.message).unwrap();
            t.fg(term::color::BLUE).unwrap();
            writeln!(t, "{:^width$}| \n", " ", width = line_width).unwrap();
            let _ = t.reset();
            t.flush().unwrap();
        }
    }
}

/// Pretty-printer.
fn print_docs(docs: &mut FixedDocs) {
    for (file, doc) in &mut docs.fixed {
        doc.iter().for_each(|doc| print_response(file, doc));
    }
}

fn check_grammar(api_key: &str, docs: &[Docs]) {
    // dbg!(api_key);
    // dbg!(docs);
    let mut docs_for_grammarly: Vec<FixedDocs> =
        docs.iter().map(|d| FixedDocs::from(d.clone())).collect();
    // dbg!(&docs_for_grammarly);
    docs_for_grammarly
        .iter_mut()
        .map(|d| docs_checked(api_key, d))
        .for_each(print_docs);
}
