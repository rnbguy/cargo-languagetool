//! The `docs` module contains all the necessary stuff to work with doc comments.

use annotate_snippets::{Level, Renderer, Snippet};
use color_eyre::{Report, Result};
use languagetool_rust::CheckResponse;
use log::debug;
use proc_macro2::{LineColumn, Literal, Span, TokenStream, TokenTree};

#[derive(Debug, Clone)]
pub struct RawDocs(Vec<Literal>);

impl From<TokenStream> for RawDocs {
    fn from(stream: TokenStream) -> Self {
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

impl RawDocs {
    /// Returns true if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DocPos {
    pub line: usize,
    pub column: usize,
}

impl From<LineColumn> for DocPos {
    fn from(span: LineColumn) -> Self {
        Self {
            line: span.line,
            column: span.column,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DocSpan {
    pub start: DocPos,
    pub end: DocPos,
}

impl From<Span> for DocSpan {
    fn from(span: Span) -> Self {
        Self {
            start: span.start().into(),
            end: span.end().into(),
        }
    }
}

/// Contains text only.
#[derive(Debug, Clone)]
pub struct Doc {
    pub text: Vec<(String, DocSpan)>,
    pub check_response: Option<CheckResponse>,
}

impl core::fmt::Display for Doc {
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

impl Doc {
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
pub struct Docs {
    pub original: RawDocs,
    pub fixed: Vec<Doc>,
}

// fn fix_string(s: &str) -> String {
//     s.replace("/// ", "")
//         .replace("//! ", "")
//         .replace(r#"\""#, r#"""#)
//         .trim_matches('\"')
//         .trim()
//         .to_owned()
// }

impl TryFrom<RawDocs> for Docs {
    type Error = Report;

    fn try_from(original: RawDocs) -> Result<Self> {
        let fixed = original.0.iter().try_fold::<_, _, Result<_>>(
            Vec::new(),
            |mut fixed_docs: Vec<Doc>, doc| {
                let (original_string, span) = {
                    let original_string: String = serde_json::from_str(&doc.to_string())?;
                    let mut span: DocSpan = doc.span().into();
                    match original_string.strip_prefix(' ') {
                        Some(fixed_string) => {
                            span.start.column += 1; // because, leading space is trimmed.
                            (fixed_string.to_owned(), span)
                        }
                        None => (original_string, span),
                    }
                };

                if let Some(last) = fixed_docs.last_mut() {
                    // If the lines are consecutive, then these two doc comments belong to a single block.

                    if let Some(last_line) = last.text.last() {
                        if span.start.line - last_line.1.end.line == 1 {
                            last.text.push((original_string, span));
                        } else {
                            fixed_docs.push(Doc {
                                text: vec![(original_string, span)],
                                check_response: None,
                            });
                        }
                    } else {
                        // unreachable!()
                    }
                } else {
                    fixed_docs.push(Doc {
                        text: vec![(original_string, span)],
                        check_response: None,
                    });
                }

                Ok(fixed_docs)
            },
        )?;

        Ok(Self { original, fixed })
    }
}
