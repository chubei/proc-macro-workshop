use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitInt, parse::{Parse, ParseStream}, Result, Token, parse_macro_input, braced};

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let _ = parse_macro_input!(input as Seq);

    let expanded = quote! {

    };

    expanded.into()
}

struct Seq;

impl Parse for Seq {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Ident>()?;
        input.parse::<Token!(in)>()?;
        input.parse::<LitInt>()?;
        input.parse::<Token!(..)>()?;
        input.parse::<LitInt>()?;
        let content;
        braced!(content in input);
        content.parse::<proc_macro2::TokenStream>()?;
        Ok(Seq)
    }
}
