//! The `docs` module contains all the necessary stuff to work with doc comments.

use annotate_snippets::{Level, Renderer, Snippet};
use color_eyre::eyre::ContextCompat;
use color_eyre::{Report, Result};
use languagetool_rust::CheckResponse;
use log::debug;

#[derive(Debug, Clone)]
pub struct Docs(pub Vec<proc_macro2::Literal>);

impl From<proc_macro2::TokenStream> for Docs {
    fn from(stream: proc_macro2::TokenStream) -> Self {
        use proc_macro2::TokenTree;

        let mut docs = vec![];
        let mut is_doc = false;
        for tree in stream {
            match tree {
                TokenTree::Ident(ident) => is_doc = ident == "doc",
                TokenTree::Group(group) => {
                    docs.append(&mut Self::from(group.stream()).0);
                }
                TokenTree::Literal(literal) => {
                    if is_doc {
                        docs.push(literal);
                    }
                }
                TokenTree::Punct(_) => {}
            };
        }
        Self(docs)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FixedDocPos {
    pub line: usize,
    pub column: usize,
}

impl From<proc_macro2::LineColumn> for FixedDocPos {
    fn from(span: proc_macro2::LineColumn) -> Self {
        Self {
            line: span.line,
            column: span.column,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FixedDocSpan {
    pub start: FixedDocPos,
    pub end: FixedDocPos,
}

impl From<proc_macro2::Span> for FixedDocSpan {
    fn from(span: proc_macro2::Span) -> Self {
        Self {
            start: span.start().into(),
            end: span.end().into(),
        }
    }
}

/// The doc is considered "fixed" when it contains text only.
#[derive(Debug, Clone)]
pub struct FixedDoc {
    pub text: Vec<(String, FixedDocSpan)>,
    pub check_response: Option<languagetool_rust::CheckResponse>,
}

impl core::fmt::Display for FixedDoc {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.text.iter();
        if let Some((first_string, _)) = iter.next() {
            write!(f, "{first_string}")?;
            for (fixed_string, _) in iter {
                write!(f, "\n{fixed_string}")?;
            }
        }
        Ok(())
    }
}

impl FixedDoc {
    /// Annotate the doc with the check response.
    ///
    /// # Errors
    /// If an error occurs.
    pub fn annotate(file: &str, source: &str, check_response: &CheckResponse) {
        debug!("Annotating: {}", file);

        check_response.matches.iter().for_each(|each_match| {
            debug!("Annotating: {:?}", each_match);

            let replacements = each_match.replacements.iter().fold(
                String::new(),
                |mut joined_string, replacement| {
                    if !joined_string.is_empty() {
                        joined_string.push_str(", ");
                    }
                    joined_string.push_str(&replacement.value);
                    joined_string
                },
            );

            let snippet = Snippet::source(&each_match.context.text)
                .line_start(
                    1 + source
                        .chars()
                        .take(each_match.offset)
                        .filter(|chr| chr == &'\n')
                        .count(),
                )
                .origin(file)
                .fold(true)
                .annotation(
                    Level::Error
                        .span(
                            each_match.context.offset
                                ..each_match.context.offset + each_match.context.length,
                        )
                        .label(&each_match.rule.description),
                )
                .annotation(
                    Level::Help
                        .span(
                            each_match.context.offset
                                ..each_match.context.offset + each_match.context.length,
                        )
                        .label(&replacements),
                );

            let message_id = format!("{}:{}", each_match.rule.id, each_match.rule.category.id);

            let message = Level::Error
                .title(&each_match.message)
                .id(&message_id)
                .snippet(snippet);

            let renderer = Renderer::styled();

            let annotation = renderer.render(message).to_string();

            println!("{annotation}");
        });
    }
}

#[derive(Debug, Clone)]
pub struct FixedDocs {
    pub original: Docs,
    pub fixed: Vec<FixedDoc>,
}

// fn fix_string(s: &str) -> String {
//     s.replace("/// ", "")
//         .replace("//! ", "")
//         .replace(r#"\""#, r#"""#)
//         .trim_matches('\"')
//         .trim()
//         .to_owned()
// }

impl TryFrom<Docs> for FixedDocs {
    type Error = Report;

    fn try_from(original: Docs) -> Result<Self> {
        let fixed =
            original
                .0
                .iter()
                .try_fold::<_, _, Result<_>>(Vec::new(), |mut fixed_docs, doc| {
                    let (original_string, span) = {
                        let original_string: String = serde_json::from_str(&doc.to_string())?;
                        let mut span: FixedDocSpan = doc.span().into();
                        match original_string.strip_prefix(' ') {
                            Some(fixed_string) => {
                                span.start.column += 1;
                                (fixed_string.to_owned(), span)
                            }
                            None => (original_string, span),
                        }
                    };

                    if fixed_docs.is_empty() {
                        fixed_docs.push(FixedDoc {
                            text: vec![(original_string, span)],
                            check_response: None,
                        });
                    } else {
                        let last = fixed_docs.last_mut().context("No last doc")?;

                        // If the lines are consecutive, then these two doc comments belong to a single block.
                        if span.start.line - last.text.last().context("must have one")?.1.end.line
                            == 1
                        {
                            last.text.push((original_string, span));
                        } else {
                            fixed_docs.push(FixedDoc {
                                text: vec![(original_string, span)],
                                check_response: None,
                            });
                        }
                    }

                    Ok(fixed_docs)
                })?;

        Ok(Self { original, fixed })
    }
}
