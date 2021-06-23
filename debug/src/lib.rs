use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _ = parse_macro_input!(input as DeriveInput);

    let expanded = quote! {};

    proc_macro::TokenStream::from(expanded)
}
