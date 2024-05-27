//! The `docs` module contains all the necessary stuff to work with doc comments.

use annotate_snippets::{Level, Renderer, Snippet};
use color_eyre::{Report, Result};
use languagetool_rust::{check::Level as LanguageToolLevel, CheckResponse};
use log::debug;
use proc_macro2::{LineColumn, Literal, Span, TokenStream, TokenTree};

use crate::{cache::Cacheable, cli::Config};

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
    /// Checks a doc.
    ///
    /// # Errors
    /// If an error occurred.
    pub fn checked<C: Cacheable>(
        &mut self,
        server: &languagetool_rust::ServerClient,
        config: &Config,
        cache: &C,
    ) -> Result<()> {
        let mut check_request =
            languagetool_rust::CheckRequest::default().with_text(self.to_string());

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

        if config.no_cache {
            self.check_response = Some({
                cache.set_and_get(&check_request, |req| {
                    Ok(rt.block_on(async { server.check(req).await })?)
                })?
            });
        } else if config.show_all || !cache.hits(&check_request)? {
            self.check_response = Some({
                cache.get_or(&check_request, |req| {
                    Ok(rt.block_on(async { server.check(req).await })?)
                })?
            });
        } else {
            // !config.no_cache && !config.show_all && cache_db.hits(&check_request)?
            // we don't print the result.
        }

        Ok(())
    }

    /// Annotate the doc with the check response.
    ///
    /// # Errors
    /// If an error occurs.
    pub fn annotate(&self, file: &str, source: &str) {
        if let Some(check_response) = self.check_response.as_ref() {
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

impl Docs {
    /// Checks docs for grammar.
    ///
    /// # Errors
    /// If an error occurs.
    pub fn checked<C: Cacheable>(
        &mut self,
        server: &languagetool_rust::ServerClient,
        config: &Config,
        cache: &C,
    ) -> Result<()> {
        for doc in &mut self.fixed {
            doc.checked(server, config, cache)?;
        }

        Ok(())
    }

    /// Transform `check_response` matches back for raw source annotation.
    ///
    /// # Errors
    /// If an error occurs.
    pub fn transform_matches(&mut self, source: &str) -> Result<()> {
        for doc in &mut self.fixed {
            let doc_str = doc.to_string();
            if let Some(check_response) = doc.check_response.as_mut() {
                for each_match in &mut check_response.matches {
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
                    let line_begin_offset = source
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
                        .zip(source.lines().skip(line_row - 1))
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

                    source[line_begin_offset..][..new_context_length]
                        .clone_into(&mut each_match.context.text);
                }
            }
        }

        Ok(())
    }

    /// Pretty-printer.
    pub fn annotate(&self, file: &str, source: &str) {
        for doc in &self.fixed {
            doc.annotate(file, source);
        }
    }
}
