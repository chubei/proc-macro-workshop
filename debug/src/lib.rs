use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let name_str = name.to_string();

    let recurse = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named.iter().map(|f| {
            let name = &f.ident;
            let name_str = f.ident.as_ref().unwrap().to_string();
            quote! {
                .field(#name_str, &self.#name)
            }
        }),
        _ => unimplemented!(),
    };

    let expanded = quote! {
        impl std::fmt::Debug for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(#name_str)
                    #(#recurse)*
                    .finish()
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}
