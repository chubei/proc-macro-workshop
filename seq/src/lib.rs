use std::ops::{Range, RangeInclusive};

use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Error, Ident, LitInt, Result, Token,
};

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let seq = parse_macro_input!(input as Seq);

    let seq_ident_name = &seq.ident.to_string();

    match seq.range {
        SeqRange::Range(range) => seq_impl(seq.body, seq_ident_name, range),
        SeqRange::RangeInclusive(range) => seq_impl(seq.body, seq_ident_name, range),
    }
    .into()
}

fn seq_impl<R: Iterator<Item = usize> + Clone>(
    stream: SeqTokenStream,
    name: &str,
    range: R,
) -> TokenStream {
    let result = if stream.has_repeat_section() {
        try_collect(stream.instantiate(name, InstMode::RepeatSection(range)))
    } else {
        try_collect(
            range
                .map(|value| stream.instantiate::<'_, Range<usize>>(name, InstMode::Whole(value)))
                .flatten(),
        )
    };

    match result {
        Ok(stream) => stream,
        Err(error) => error.into_compile_error(),
    }
}

fn try_collect<T: FromIterator<TokenTree>>(
    iter: impl Iterator<Item = Result<TokenTree>>,
) -> Result<T> {
    let mut result = vec![];
    for token in iter {
        match token {
            Ok(token) => result.push(token),
            Err(error) => return Err(error),
        }
    }
    Ok(result.into_iter().collect())
}

#[derive(Debug)]
struct SeqIdent {
    prefix: Option<Ident>,
    ident: Ident,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeqDelimiter {
    RepeatSection,
    Delimiter(Delimiter),
}

#[derive(Debug)]
struct SeqGroup {
    delimiter: SeqDelimiter,
    stream: SeqTokenStream,
}

struct SeqParseStream {
    tokens: Vec<TokenTree>,
    index: usize,
}

const PASTE_TOKEN: char = '~';
const REPEAT_SECTION_START_PUNCT: char = '#';
const REPEAT_SECTION_END_PUNCT: char = '*';

impl SeqParseStream {
    fn from_token_stream(stream: TokenStream) -> Self {
        let tokens = stream.into_iter().collect();
        Self { tokens, index: 0 }
    }

    fn is_empty(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn peek_paste_token(&self) -> bool {
        if self.index + 1 >= self.tokens.len() {
            false
        } else {
            match &self.tokens[self.index + 1] {
                TokenTree::Punct(p) => p.as_char() == PASTE_TOKEN,
                _ => false,
            }
        }
    }

    fn peek_repeat_section(&self) -> bool {
        if self.index + 2 >= self.tokens.len() {
            return false;
        }
        let punct = match &self.tokens[self.index] {
            TokenTree::Punct(punct) => punct,
            _ => return false,
        };
        if punct.as_char() != REPEAT_SECTION_START_PUNCT {
            return false;
        }
        let group = match &self.tokens[self.index + 1] {
            TokenTree::Group(group) => group,
            _ => return false,
        };
        if group.delimiter() != Delimiter::Parenthesis {
            return false;
        }
        let punct = match &self.tokens[self.index + 2] {
            TokenTree::Punct(punct) => punct,
            _ => return false,
        };
        punct.as_char() == REPEAT_SECTION_END_PUNCT
    }

    fn peek_group(&self) -> bool {
        if self.index >= self.tokens.len() {
            false
        } else {
            if let TokenTree::Group(_) = self.tokens[self.index] {
                true
            } else {
                false
            }
        }
    }

    fn peek_ident(&self) -> bool {
        if self.index >= self.tokens.len() {
            false
        } else {
            if let TokenTree::Ident(_) = self.tokens[self.index] {
                true
            } else {
                false
            }
        }
    }

    fn peek_punct(&self) -> bool {
        if self.index >= self.tokens.len() {
            false
        } else {
            if let TokenTree::Punct(_) = self.tokens[self.index] {
                true
            } else {
                false
            }
        }
    }

    fn parse_seq_group(&mut self) -> Result<SeqGroup> {
        if self.peek_repeat_section() {
            self.index += 1;
            let group = self.parse_seq_group()?;
            self.parse_punct()?;
            Ok(SeqGroup {
                delimiter: SeqDelimiter::RepeatSection,
                stream: group.stream,
            })
        } else {
            if self.index >= self.tokens.len() {
                Err(Error::new(Span::mixed_site(), "Expecting group, got EOF"))
            } else {
                match &self.tokens[self.index] {
                    TokenTree::Group(group) => {
                        self.index += 1;
                        let delimiter = group.delimiter();
                        let mut stream = SeqParseStream::from_token_stream(group.stream());
                        let stream = SeqTokenStream::seq_parse(&mut stream)?;
                        Ok(SeqGroup {
                            delimiter: SeqDelimiter::Delimiter(delimiter),
                            stream,
                        })
                    }
                    other => Err(Error::new(other.span(), "Expecting group")),
                }
            }
        }
    }

    fn parse_seq_ident(&mut self) -> Result<SeqIdent> {
        if self.peek_paste_token() {
            let prefix = self.parse_ident()?;
            self.parse_punct()?;
            let ident = self.parse_ident()?;
            Ok(SeqIdent {
                prefix: Some(prefix),
                ident,
            })
        } else {
            let ident = self.parse_ident()?;
            Ok(SeqIdent {
                prefix: None,
                ident,
            })
        }
    }

    fn parse_ident(&mut self) -> Result<Ident> {
        if self.index >= self.tokens.len() {
            Err(Error::new(
                Span::mixed_site(),
                "Expecting identifier, got EOF",
            ))
        } else {
            match &self.tokens[self.index] {
                TokenTree::Ident(ident) => {
                    self.index += 1;
                    Ok(ident.clone())
                }
                other => Err(Error::new(other.span(), "Expecting identifier")),
            }
        }
    }

    fn parse_punct(&mut self) -> Result<Punct> {
        if self.index >= self.tokens.len() {
            Err(Error::new(
                Span::mixed_site(),
                "Expecting punctuation, got EOF",
            ))
        } else {
            match &self.tokens[self.index] {
                TokenTree::Punct(punct) => {
                    self.index += 1;
                    Ok(punct.clone())
                }
                other => Err(Error::new(other.span(), "Expecting punctuation")),
            }
        }
    }

    fn parse_literal(&mut self) -> Result<Literal> {
        if self.index >= self.tokens.len() {
            Err(Error::new(Span::mixed_site(), "Expecting literal, got EOF"))
        } else {
            match &self.tokens[self.index] {
                TokenTree::Literal(literal) => {
                    self.index += 1;
                    Ok(literal.clone())
                }
                other => Err(Error::new(other.span(), "Expecting literal")),
            }
        }
    }
}

impl Parse for SeqParseStream {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self::from_token_stream(input.parse()?))
    }
}

#[derive(Debug)]
enum SeqTokenTree {
    SeqGroup(SeqGroup),
    SeqIdent(SeqIdent),
    Punct(Punct),
    Literal(Literal),
}

impl SeqTokenTree {
    fn has_repeat_section(&self) -> bool {
        if let Self::SeqGroup(group) = self {
            if group.delimiter == SeqDelimiter::RepeatSection {
                true
            } else {
                group.stream.has_repeat_section()
            }
        } else {
            false
        }
    }

    fn instantiate<R: Iterator<Item = usize> + Clone>(
        &self,
        name: &str,
        mode: InstMode<R>,
    ) -> Vec<Result<TokenTree>> {
        match self {
            Self::SeqGroup(group) => match group.delimiter {
                SeqDelimiter::Delimiter(delimiter) => {
                    let stream = try_collect(group.stream.instantiate(name, mode));
                    vec![stream.map(|stream| TokenTree::Group(Group::new(delimiter, stream)))]
                }
                SeqDelimiter::RepeatSection => {
                    let range = match mode {
                        InstMode::RepeatSection(range) => range,
                        _ => {
                            return vec![Err(Error::new(
                                Span::call_site(),
                                "Repeat sections cannot be nested",
                            ))]
                        }
                    };
                    range
                        .map(|value| {
                            group
                                .stream
                                .instantiate::<'_, Range<usize>>(name, InstMode::Whole(value))
                        })
                        .flatten()
                        .collect()
                }
            },
            Self::SeqIdent(SeqIdent { prefix, ident }) => {
                if ident == name {
                    match mode {
                        InstMode::RepeatSection(_) => vec![Err(Error::new(
                            ident.span(),
                            "Seq identifier out of repeat section",
                        ))],
                        InstMode::Whole(value) => {
                            if let Some(prefix) = prefix {
                                vec![Ok(TokenTree::Ident(Ident::new(
                                    &format!("{}{}", prefix, value),
                                    prefix.span(),
                                )))]
                            } else {
                                vec![Ok(TokenTree::Literal(Literal::usize_unsuffixed(value)))]
                            }
                        }
                    }
                } else {
                    let mut result = if let Some(prefix) = prefix {
                        vec![
                            Ok(TokenTree::Ident(prefix.clone())),
                            Ok(TokenTree::Punct(Punct::new(PASTE_TOKEN, Spacing::Alone))),
                        ]
                    } else {
                        vec![]
                    };
                    result.push(Ok(TokenTree::Ident(ident.clone())));
                    result
                }
            }
            Self::Punct(punct) => vec![Ok(TokenTree::Punct(punct.clone()))],
            Self::Literal(literal) => vec![Ok(TokenTree::Literal(literal.clone()))],
        }
    }
}

#[derive(Debug, Clone)]
enum InstMode<R: Iterator<Item = usize>> {
    RepeatSection(R),
    Whole(usize),
}

#[derive(Debug)]
struct SeqTokenStream {
    tokens: Vec<SeqTokenTree>,
}

impl SeqTokenStream {
    fn seq_parse(input: &mut SeqParseStream) -> Result<Self> {
        let mut tokens = vec![];

        while !input.is_empty() {
            if input.peek_paste_token() || input.peek_ident() {
                tokens.push(SeqTokenTree::SeqIdent(input.parse_seq_ident()?));
            } else if input.peek_repeat_section() || input.peek_group() {
                tokens.push(SeqTokenTree::SeqGroup(input.parse_seq_group()?));
            } else if input.peek_punct() {
                tokens.push(SeqTokenTree::Punct(input.parse_punct()?));
            } else {
                tokens.push(SeqTokenTree::Literal(input.parse_literal()?));
            }
        }

        Ok(SeqTokenStream { tokens })
    }

    fn has_repeat_section(&self) -> bool {
        for token in &self.tokens {
            if token.has_repeat_section() {
                return true;
            }
        }
        false
    }

    fn instantiate<'a, R: Iterator<Item = usize> + Clone + 'a>(
        &'a self,
        name: &'a str,
        mode: InstMode<R>,
    ) -> impl Iterator<Item = Result<TokenTree>> + 'a {
        self.tokens
            .iter()
            .map(move |token| token.instantiate(name, mode.clone()))
            .flatten()
    }
}

impl Parse for SeqTokenStream {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut stream = input.parse::<SeqParseStream>()?;
        Self::seq_parse(&mut stream)
    }
}

#[derive(Debug)]
enum SeqRange {
    Range(Range<usize>),
    RangeInclusive(RangeInclusive<usize>),
}

#[derive(Debug)]
enum SeqRangeKind {
    Range,
    RangeInclusive,
}

#[derive(Debug)]
struct Seq {
    ident: Ident,
    range: SeqRange,
    body: SeqTokenStream,
}

impl Parse for Seq {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse::<Ident>()?;

        input.parse::<Token!(in)>()?;

        let lower = input.parse::<LitInt>()?.base10_parse()?;

        let range_kind = {
            let lookahead1 = input.lookahead1();
            if lookahead1.peek(Token!(..=)) {
                input.parse::<Token!(..=)>()?;
                SeqRangeKind::RangeInclusive
            } else if lookahead1.peek(Token!(..)) {
                input.parse::<Token!(..)>()?;
                SeqRangeKind::Range
            } else {
                return Err(lookahead1.error());
            }
        };

        let upper = input.parse::<LitInt>()?.base10_parse()?;

        let range = match range_kind {
            SeqRangeKind::Range => SeqRange::Range(lower..upper),
            SeqRangeKind::RangeInclusive => SeqRange::RangeInclusive(lower..=upper),
        };

        let content;
        braced!(content in input);
        let body = content.parse::<SeqTokenStream>()?;
        Ok(Seq { ident, range, body })
    }
}
