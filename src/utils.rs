use std::path::PathBuf;

use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use log::debug;

use crate::doc::{Docs, FixedDoc, FixedDocs};

/// Reads the .rs files in the directory recursively.
pub fn fetch_docs(dir: &PathBuf) -> Result<Vec<Docs>> {
    use proc_macro2::TokenStream;

    // dbg!(dir);

    let is_rs = |e: &walkdir::DirEntry| -> bool {
        e.file_type().is_file()
            && e.path()
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("rs"))
    };
    let parse_docs = |path: &String| -> Result<Docs> {
        use std::fs;
        let content = fs::read_to_string(path)?;
        let stream: TokenStream = syn::parse_str(&content)?;
        // dbg!(&stream);
        Ok(Docs::from((path, stream)))
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
        .filter(|d| d.as_ref().map(|x| x.0.is_empty()).ok() == Some(false))
        .collect::<Result<_>>()
}

fn doc_checked<'a>(server: &languagetool_rust::ServerClient, doc: &'a mut FixedDoc) -> Result<()> {
    let check_request = languagetool_rust::CheckRequest::default()
        .with_text(doc.formatted_string())
        .with_language("en-US".to_owned());

    let rt = tokio::runtime::Runtime::new()?;

    doc.check_response = Some(rt.block_on(async { server.check(&check_request).await })?);

    Ok(())
}

fn docs_checked(server: &languagetool_rust::ServerClient, docs: &mut FixedDocs) -> Result<()> {
    for docs in docs.fixed.values_mut() {
        for doc in docs {
            doc_checked(server, doc)?;
        }
    }

    Ok(())
}

/// Pretty-printer.
fn print_docs(docs: &FixedDocs) -> Result<()> {
    for (file, docs) in &docs.fixed {
        let file_str = std::fs::read_to_string(file)?;
        for doc in docs {
            let check_response = doc.check_response.as_ref().context("No check response")?;
            if !check_response.matches.is_empty() {
                debug!("Annotating: {}", file);
                println!("{}", check_response.annotate(&file_str, Some(file), true));
            }
        }
    }

    Ok(())
}

fn transform_matches(docs: &mut FixedDocs) -> Result<()> {
    for (file, docs) in &mut docs.fixed {
        for doc in docs {
            let doc_str = doc.formatted_string();
            let check_response = doc.check_response.as_mut().context("No check response")?;
            for each_match in &mut check_response.matches {
                let file_str = std::fs::read_to_string(file)?;

                assert_eq!(each_match.length, each_match.context.length);

                let context_offset = each_match.context.offset; // this gets changed too
                let context_text = &each_match.context.text; // this gets trims

                let length = each_match.context.length; // stays same
                let offset = each_match.offset; // this changes

                let row = doc_str.chars().take(offset).filter(|&c| c == '\n').count();

                let (line, span) = &doc.text[row];

                let splits = doc_str.splitn(row + 1, '\n').collect::<Vec<_>>();
                let line_offset = offset
                    - row
                    - splits
                        .iter()
                        .rev()
                        .skip(1)
                        .map(|st| st.len())
                        .sum::<usize>();

                let line_row = span.start.line;
                let line_offset = span.start.column + 3 + line_offset;
                let line_begin_offset = line_row - 1
                    + file_str
                        .lines()
                        .take(line_row - 1)
                        .map(str::len)
                        .sum::<usize>();

                let line_length = file_str.lines().nth(line_row - 1).unwrap().len();

                let context_text = context_text
                    .strip_prefix("...")
                    .unwrap_or(context_text)
                    .strip_suffix("...")
                    .unwrap_or(context_text);
                let context_length = context_text.len();

                let new_context_offset = line_offset;

                let new_context_length = if line_length < new_context_offset + context_length {
                    line_length
                } else {
                    context_length
                };

                // updating value
                each_match.offset = line_begin_offset + line_offset; // this changes

                each_match.context.offset = new_context_offset; // this gets changed too

                file_str[line_begin_offset + line_offset - new_context_offset..]
                    [..new_context_length]
                    .clone_into(&mut each_match.context.text); // this gets trims

                // let length = each_match.context.length; // stays same
            }
        }
    }

    Ok(())
}

pub fn check_grammar(server: &languagetool_rust::ServerClient, docs: &[Docs]) -> Result<()> {
    for doc in docs {
        let mut fixed_doc = FixedDocs::try_from(doc.clone())?;
        docs_checked(server, &mut fixed_doc)?;
        transform_matches(&mut fixed_doc)?;
        print_docs(&fixed_doc)?;
    }

    println!("Checked {} files.", docs.len());

    Ok(())
}
