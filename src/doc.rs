//! The `docs` module contains all the necessary stuff to work with doc comments.

use std::collections::HashMap;

use annotate_snippets::{Level, Renderer, Snippet};
use color_eyre::eyre::ContextCompat;
use color_eyre::{Report, Result};
use log::debug;

// Ideally error should be printed that way:
//
// error[grammar]: Missing subject
// --> src/main.rs:138:16
//     |
// 138 | /// Reads the .rs files in the directory recursively.
//     |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//     |     - This sentence appears to be missing a subject. Consider adding a subject or rewriting the sentence.
//
//
//
// error[grammar]: Spelling
// --> src/main.rs:138:16
//     |
// 138 | /// Thisf module is for easing the pain with printing text in the terminal.
//     |     ^^^^^
//     |     - The word "Thisf" is not in our dictionary. If you are sure this spelling is correct,
//     |     - you can add it to your personal dictionary to prevent future alerts.
//

#[derive(Debug, Clone)]
pub struct Docs(pub HashMap<String, Vec<proc_macro2::Literal>>);

impl Docs {
    fn append(&mut self, docs: Self) {
        for (k, mut v) in docs.0 {
            self.0.entry(k).or_default().append(&mut v);
        }
    }
}

impl<T> From<(T, proc_macro2::TokenStream)> for Docs
where
    T: AsRef<str>,
{
    fn from(stream: (T, proc_macro2::TokenStream)) -> Self {
        use proc_macro2::TokenTree;

        let mut docs = Self(HashMap::new());
        let mut is_doc = false;
        for tree in stream.1 {
            match tree {
                TokenTree::Ident(ident) => is_doc = ident == "doc",
                TokenTree::Group(group) => {
                    docs.append(Self::from((stream.0.as_ref().to_owned(), group.stream())));
                }
                TokenTree::Literal(literal) => {
                    if is_doc {
                        docs.0
                            .entry(stream.0.as_ref().to_owned())
                            .or_default()
                            .push(literal);
                    }
                }
                TokenTree::Punct(_) => {}
            };
        }
        docs
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

impl std::fmt::Display for FixedDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    pub fn annotate(&self, file: &str) -> Result<()> {
        let source = std::fs::read_to_string(file)?;

        let check_response = self.check_response.as_ref().context("No check response")?;
        debug!("Annotating: {}", file);

        for each_match in &check_response.matches {
            debug!("Annotating: {:?}", each_match);

            let replacements = each_match
                .replacements
                .iter()
                .fold(String::new(), |mut acc, r| {
                    if !acc.is_empty() {
                        acc.push_str(", ");
                    }
                    acc.push_str(&r.value);
                    acc
                });

            let snippet = Snippet::source(&source)
                .line_start(
                    1 + source
                        .chars()
                        .take(each_match.offset)
                        .filter(|c| *c == '\n')
                        .count(),
                )
                .origin(file)
                .fold(true)
                .annotation(
                    Level::Error
                        .span(each_match.offset..each_match.offset + each_match.length)
                        .label(&each_match.rule.description),
                )
                .annotation(
                    Level::Help
                        .span(each_match.offset..each_match.offset + each_match.length)
                        .label(&replacements),
                );

            let message = Level::Error
                .title(&each_match.message)
                .id(&each_match.rule.id)
                .snippet(snippet);

            let renderer = Renderer::styled();

            println!("{}", renderer.render(message));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FixedDocs {
    pub original: Docs,
    pub fixed: HashMap<String, Vec<FixedDoc>>,
    // mappings: Vec<usize>,
}

// fn fix_string(s: &str) -> String {
//     s.replace("/// ", "")
//         .replace("//! ", "")
//         .replace(r#"\""#, r#"""#)
//         .trim_matches('\"')
//         .trim()
//         .to_owned()
// }

// impl<T> From<T> for FixedDocs where T: AsRef<Docs> {
//     fn from(original: T) -> FixedDocs {
impl TryFrom<Docs> for FixedDocs {
    type Error = Report;

    fn try_from(original: Docs) -> Result<Self> {
        // let mut fixed = Docs(HashMap::new());
        // let mut mappings = Vec::new();
        // let original = original.as_ref();
        let mut fixed: HashMap<String, Vec<FixedDoc>> = HashMap::new();

        for (file, docs) in &original.0 {
            for doc in docs {
                // let fixed_string = &fix_string(&original_string);
                // dbg!(&original_string, &fixed_string);
                // let start_column_diff = original_string.len() - fixed_string.len();
                // span.start.column += start_column_diff;
                // dbg!(&original_string, &fixed_string, &span);

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

                if fixed.contains_key(file) {
                    let fixed_docs = fixed.get_mut(file).context("No doc")?;
                    let last = fixed_docs.last_mut().context("No last doc")?;

                    // If the lines are consecutive, then these two doc comments belong to a single block.
                    if span.start.line - last.text.last().context("must have one")?.1.end.line == 1
                    {
                        last.text.push((original_string.clone(), span));
                    } else {
                        fixed_docs.push(FixedDoc {
                            text: vec![(original_string.clone(), span)],
                            check_response: None,
                        });
                    }
                } else {
                    fixed.insert(
                        file.clone(),
                        vec![FixedDoc {
                            text: vec![(original_string.clone(), span)],
                            check_response: None,
                        }],
                    );
                }
            }
        }

        Ok(Self {
            original,
            fixed,
            // mappings,
        })
    }
}
