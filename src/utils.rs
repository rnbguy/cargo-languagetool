use std::path::PathBuf;

use color_eyre::Result;

use crate::cache::Cacheable;
use crate::cli::Config;
use crate::doc::{Docs, RawDocs};
use proc_macro2::TokenStream;

/// Reads the .rs files in the directory recursively.
///
/// # Errors
/// If an error occurs.
pub fn fetch_docs(dir: &PathBuf) -> Result<Vec<(String, RawDocs)>> {
    walkdir::WalkDir::new(dir)
        .max_depth(999)
        .into_iter()
        .filter_map(Result::ok)
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
            Ok((path, RawDocs::from(stream)))
        })
        .filter(|result| result.as_ref().map(|(_, docs)| !docs.is_empty()).ok() != Some(true))
        .collect::<Result<_>>()
}

/// Check the grammar of the documents and annotates the results.
///
/// # Errors
/// If an error occurs.
pub fn check_and_annotate<I: IntoIterator<Item = (String, RawDocs)>, C: Cacheable>(
    server: &languagetool_rust::ServerClient,
    config: &Config,
    files: I,
    cache: &C,
) -> Result<()> {
    for (file, doc) in files {
        let mut docs = Docs::try_from(doc)?;
        docs.checked(server, config, cache)?;

        let source = std::fs::read_to_string(&file)?;

        docs.transform_matches(&source);
        docs.annotate(&file, &source);
    }

    Ok(())
}

// fn fix_string(s: &str) -> String {
//     s.replace("/// ", "")
//         .replace("//! ", "")
//         .replace(r#"\""#, r#"""#)
//         .trim_matches('\"')
//         .trim()
//         .to_owned()
// }
