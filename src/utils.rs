use std::path::PathBuf;

use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use languagetool_rust::check::Level as LanguageToolLevel;

use crate::cache::DB;
use crate::cli::Config;
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

fn doc_checked(
    server: &languagetool_rust::ServerClient,
    config: &Config,
    doc: &mut FixedDoc,
) -> Result<()> {
    let mut check_request = languagetool_rust::CheckRequest::default().with_text(doc.to_string());

    if let (Some(username), Some(api_key)) = (&config.username, &config.api_key) {
        check_request.username = Some(username.clone());
        check_request.api_key = Some(api_key.clone());
    }

    check_request.language.clone_from(&config.language);

    if config.picky {
        check_request.level = LanguageToolLevel::Picky;
    }

    check_request.enabled_categories = Some(
        config
            .enable_categories
            .iter()
            .map(ToString::to_string)
            .collect(),
    );

    check_request.enabled_rules = Some(
        config
            .enable_rules
            .iter()
            .map(ToString::to_string)
            .collect(),
    );

    check_request.disabled_categories = Some(
        config
            .disable_categories
            .iter()
            .map(ToString::to_string)
            .collect(),
    );

    check_request.disabled_rules = Some(
        config
            .disable_rules
            .iter()
            .map(ToString::to_string)
            .collect(),
    );

    check_request.enabled_only = config.enable_only;

    // doc.check_response = Some(
    //     tokio::runtime::Runtime::new()?.block_on(async { server.check(&check_request).await })?,
    // );

    let cache_db = DB::new()?;
    doc.check_response = Some(cache_db.get_or(&check_request, |req| {
        Ok(tokio::runtime::Runtime::new()?.block_on(async { server.check(req).await })?)
    })?);

    Ok(())
}

fn docs_checked(
    server: &languagetool_rust::ServerClient,
    config: &Config,
    docs: &mut FixedDocs,
) -> Result<()> {
    for docs in docs.fixed.values_mut() {
        for doc in docs {
            doc_checked(server, config, doc)?;
        }
    }

    Ok(())
}

/// Pretty-printer.
fn print_docs(docs: &FixedDocs) -> Result<()> {
    for (file, docs) in &docs.fixed {
        for doc in docs {
            doc.annotate(file)?;
        }
    }

    Ok(())
}

fn transform_matches(docs: &mut FixedDocs) -> Result<()> {
    for (file, docs) in &mut docs.fixed {
        for doc in docs {
            let doc_str = doc.to_string();
            let check_response = doc.check_response.as_mut().context("No check response")?;
            for each_match in &mut check_response.matches {
                let file_str = std::fs::read_to_string(file)?;

                assert_eq!(each_match.length, each_match.context.length);

                let row = doc_str
                    .chars()
                    .take(each_match.offset)
                    .filter(|&c| c == '\n')
                    .count();

                let (_line, span) = &doc.text[row];

                let doc_prev_line_end_offset = doc_str
                    .lines()
                    .take(row)
                    .map(|st| st.len() + 1) // because of extra space
                    .sum::<usize>();

                // offset of the match in the line in doc comments
                let doc_line_offset = each_match.offset - doc_prev_line_end_offset;

                let line_row = span.start.line;
                let line_offset = span.start.column + 3 + doc_line_offset; // because of rust comment tags

                // line beginning in the file
                let line_begin_offset = line_row - 1
                    + file_str
                        .lines()
                        .take(line_row - 1)
                        .map(str::len)
                        .sum::<usize>();

                let doc_match_offset = each_match.offset;

                // updating value
                each_match.offset = line_begin_offset + line_offset;

                // LT context starts at: each_match.offset - each_match.context.offset
                // start the context from the same line as the beginning of the match.
                each_match.context.offset = line_offset; // this gets changed too

                // end the context at the end of the line of the end of the match.

                let mut new_context_length = 0;
                let mut length_delta = 0;
                let mut match_count = doc_prev_line_end_offset;

                for (doc_line, file_line) in doc_str
                    .lines()
                    .skip(row)
                    .zip(file_str.lines().skip(line_row - 1))
                {
                    new_context_length += file_line.len() + 1; // because of newline
                    match_count += doc_line.len() + 1; // because of newline
                    if doc_match_offset + each_match.length < match_count {
                        break;
                    }
                    length_delta += span.start.column + 3; // because of rust comment tags
                }

                each_match.length += length_delta;
                each_match.context.length = each_match.length;

                file_str[line_begin_offset..][..new_context_length]
                    .clone_into(&mut each_match.context.text);
            }
        }
    }

    Ok(())
}

pub fn check_grammar(
    server: &languagetool_rust::ServerClient,
    config: &Config,
    docs: &[Docs],
) -> Result<()> {
    for doc in docs {
        let mut fixed_doc = FixedDocs::try_from(doc.clone())?;
        docs_checked(server, config, &mut fixed_doc)?;
        transform_matches(&mut fixed_doc)?;
        print_docs(&fixed_doc)?;
    }

    Ok(())
}
