use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let vis = input.vis;

    // Generate builder struct name.
    let builder_name = format_ident!("{}Builder", name);
    // Generate builder fields.
    let builder_fields = builder_fields(&input.data);
    // Generate setters for all the fields.
    let builder_setters = builder_setters(&input.data);
    // Generate build method.
    let builder_build = builder_build(&name, &input.data);

    let expanded = quote! {
        #[derive(Default)]
        #vis struct #builder_name {
            #builder_fields
        }

        impl #builder_name {
            #builder_setters

            #builder_build
        }

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name::default()
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

// Generated code looks like this:
// ```rust
// executable: Option<String>,
// args: Option<Vec<String>>,
// ```
fn builder_fields(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote! {
                        #name: Option<#ty>,
                    }
                });
                quote! {
                    #(#recurse)*
                }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

// Generated code looks like this:
// ```rust
// pub fn executable(&mut self, executable: String) -> Self {
//     self.executable = Some(executable);
//     self
// }
// pub fn args(&mut self, args: Vec<String>) -> Self {
//     self.args = Some(args);
//     self
// }
// ```
fn builder_setters(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote! {
                        pub fn #name(&mut self, #name: #ty) -> &mut Self {
                            self.#name = Some(#name);
                            self
                        }
                    }
                });
                quote! {
                    #(#recurse)*
                }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

// Generated code looks like this:
// ```rust
// pub fn build(&mut self) -> Result<Command, &'static str> {
//     Ok(Command {
//         executable: self.executable.take().ok_or("Missing field executable")?,
//         args: self.args.take().ok_or("Missing field args")?,
//     })
// }
// ```
fn builder_build(name: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let message = format!("Missing field {}", name.as_ref().unwrap().to_string());
                    quote! {
                        #name: self.#name.take().ok_or(#message)?,
                    }
                });
                quote! {
                    pub fn build(&mut self) -> Result<#name, &'static str> {
                        Ok(#name {
                            #(#recurse)*
                        })
                    }
                }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}
