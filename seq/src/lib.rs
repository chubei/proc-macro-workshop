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
        result.extend(process_token_stream(
            seq.body.clone(),
            &seq.ident.to_string(),
            value,
        ));
    }

    result.into()
}

fn process_token_stream(stream: TokenStream, name: &str, value: usize) -> TokenStream {
    let mut result: Vec<TokenTree> = vec![];
    let tokens: Vec<_> = stream.into_iter().collect();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Group(group) => {
                let stream = process_token_stream(group.stream(), name, value);
                result.push(TokenTree::Group(Group::new(group.delimiter(), stream)));
            }
            TokenTree::Ident(ident) => {
                // Unprefixed.
                if ident == name {
                    result.push(TokenTree::Literal(Literal::usize_unsuffixed(value)));
                    i += 1;
                    continue;
                }
                // Look forward.
                if let TokenTree::Punct(punct) = &tokens[i + 1] {
                    if punct.as_char() == '~' {
                        if let TokenTree::Ident(ident2) = &tokens[i + 2] {
                            if ident2 == name {
                                let ident = Ident::new(
                                    &format!("{}{}", ident.to_string(), value),
                                    ident.span(),
                                );
                                result.push(TokenTree::Ident(ident));
                                i += 3;
                                continue;
                            }
                        }
                    }
                }
                result.push(TokenTree::Ident(ident.clone()));
            }
            other => result.push(other.clone()),
        }

        i += 1;
    }
    result.into_iter().collect()
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
