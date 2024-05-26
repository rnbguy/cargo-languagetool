use std::path::PathBuf;

use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use languagetool_rust::check::Level as LanguageToolLevel;

use crate::cache::sled::SledCacheStore as SledCacheDb;
use crate::cache::Cacheable;
use crate::cli::Config;
use crate::doc::{Docs, FixedDoc, FixedDocs};

/// Reads the .rs files in the directory recursively.
///
/// # Errors
/// If an error occurs.
pub fn fetch_docs(dir: &PathBuf) -> Result<Vec<Docs>> {
    use proc_macro2::TokenStream;

    walkdir::WalkDir::new(dir)
        .max_depth(999)
        .into_iter()
        .filter_map(core::result::Result::ok)
        .filter(|entry| {
            !entry.file_type().is_dir()
                && entry
                    .path()
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("rs"))
        })
        .filter_map(|entry| Some(entry.path().to_str()?.to_owned()))
        .map(|path| {
            let content = std::fs::read_to_string(&path)?;
            let stream: TokenStream = syn::parse_str(&content)?;
            Ok(Docs::from((&path, stream)))
        })
        .filter(|docs| docs.as_ref().map(Docs::is_empty).ok() != Some(true))
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

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()?;

    // doc.check_response = Some(rt.block_on(async { server.check(&check_request).await })?);

    let cargo_languagetool_project_dir =
        directories::ProjectDirs::from("rnbguy", "github", "cargo-languagetool")
            .context("failed to get cache directory")?;
    let cache_db = SledCacheDb::new(cargo_languagetool_project_dir.cache_dir())?;

    if config.no_cache {
        doc.check_response = Some({
            cache_db.set_and_get(&check_request, |req| {
                Ok(rt.block_on(async { server.check(req).await })?)
            })?
        });
    } else if config.show_all || !cache_db.hits(&check_request)? {
        doc.check_response = Some({
            cache_db.get_or(&check_request, |req| {
                Ok(rt.block_on(async { server.check(req).await })?)
            })?
        });
    } else {
        // !config.no_cache && !config.show_all && cache_db.hits(&check_request)?
        // we don't print the result.
    }

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
fn print_docs(file_docs: &FixedDocs) -> Result<()> {
    for (file, docs) in &file_docs.fixed {
        let source = std::fs::read_to_string(file)?;
        for doc in docs {
            if let Some(check_response) = &doc.check_response {
                FixedDoc::annotate(file, &source, check_response);
            }
        }
    }

    Ok(())
}

fn transform_matches(docs: &mut FixedDocs) -> Result<()> {
    for (file, docs) in &mut docs.fixed {
        for doc in docs {
            let doc_str = doc.to_string();
            if let Some(check_response) = doc.check_response.as_mut() {
                for each_match in &mut check_response.matches {
                    let file_str = std::fs::read_to_string(file)?;

                    // ensured by LT API.
                    // assert_eq!(each_match.length, each_match.context.length);

                    let row = doc_str
                        .chars()
                        .take(each_match.offset)
                        .filter(|chr| chr == &'\n')
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
                    let line_begin_offset = file_str
                        .lines()
                        .take(line_row - 1)
                        .map(|st| st.len() + 1)
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
    }

    Ok(())
}

/// Check the grammar of the documents.
///
/// # Errors
/// If an error occurs.
pub fn check_grammar<I: IntoIterator<Item = Docs>>(
    server: &languagetool_rust::ServerClient,
    config: &Config,
    docs: I,
) -> Result<()> {
    for doc in docs {
        let mut fixed_doc = FixedDocs::try_from(doc)?;
        docs_checked(server, config, &mut fixed_doc)?;
        transform_matches(&mut fixed_doc)?;
        print_docs(&fixed_doc)?;
    }

    Ok(())
}
