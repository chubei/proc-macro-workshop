use proc_macro2::{Group, Literal, TokenStream, TokenTree};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, LitInt, Result, Token,
};

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let seq = parse_macro_input!(input as Seq);

    let mut result = TokenStream::new();
    for value in seq.start..seq.end {
        result.extend(
            seq.body
                .clone()
                .into_iter()
                .map(|token| replace_ident_recursive(token, &seq.ident.to_string(), value)),
        );
    }

    result.into()
}

fn replace_ident_recursive(token: TokenTree, name: &str, value: usize) -> TokenTree {
    match token {
        TokenTree::Group(group) => {
            let stream: TokenStream = group
                .stream()
                .into_iter()
                .map(|token| replace_ident_recursive(token, name, value))
                .collect();
            TokenTree::Group(Group::new(group.delimiter(), stream))
        }
        TokenTree::Ident(ident) => {
            if ident == name {
                TokenTree::Literal(Literal::usize_unsuffixed(value))
            } else {
                TokenTree::Ident(ident)
            }
        }
        other => other,
    }
}

#[derive(Debug)]
struct Seq {
    ident: Ident,
    start: usize,
    end: usize,
    body: TokenStream,
}

impl Parse for Seq {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse::<Ident>()?;
        input.parse::<Token!(in)>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Token!(..)>()?;
        let end = input.parse::<LitInt>()?.base10_parse()?;
        let content;
        braced!(content in input);
        let body = content.parse::<TokenStream>()?;
        Ok(Seq {
            ident,
            start,
            end,
            body,
        })
    }
}
