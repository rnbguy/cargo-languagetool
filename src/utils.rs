use crate::doc::{Docs, FixedDoc, FixedDocs};

use color_eyre::{eyre::ContextCompat, Result};

/// Reads the .rs files in the directory recursively.
pub fn fetch_docs(dir: &str) -> Result<Vec<Docs>> {
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

async fn doc_checked<'a>(
    server: &languagetool_rust::ServerClient,
    doc: &'a mut FixedDoc,
) -> Result<()> {
    let check_request = languagetool_rust::CheckRequest::default()
        .with_text(doc.text.clone())
        .with_language("en-US".to_owned());

    doc.check_response = Some(server.check(&check_request).await?);

    Ok(())
}

async fn docs_checked<'a>(
    server: &languagetool_rust::ServerClient,
    docs: &'a mut FixedDocs,
) -> Result<()> {
    for docs in docs.fixed.values_mut() {
        for doc in docs {
            doc_checked(server, doc).await?;
        }
    }

    Ok(())
}

/// Pretty-printer.
fn print_docs(docs: &FixedDocs) -> Result<()> {
    for (file, docs) in &docs.fixed {
        for doc in docs {
            let check_response = doc.check_response.as_ref().context("No check response")?;
            if !check_response.matches.is_empty() {
                println!("{}", check_response.annotate(&doc.text, Some(file), true));
            }
        }
    }

    Ok(())
}

pub async fn check_grammar(server: &languagetool_rust::ServerClient, docs: &[Docs]) -> Result<()> {
    for doc in docs {
        let mut fixed_doc = FixedDocs::try_from(doc.clone())?;
        docs_checked(server, &mut fixed_doc).await?;
        print_docs(&fixed_doc)?;
    }

    Ok(())
}
