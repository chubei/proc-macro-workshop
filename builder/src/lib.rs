use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, GenericArgument, Ident, PathArguments, Type,
};

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

// If `ty` is an `Option<T>`, return `Option<ty for T>`, otherwise return `None`.
fn extract_inner_if_option(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(path) => match path.qself {
            Some(_) => None,
            None => {
                let segments = &path.path.segments;
                if segments.len() == 1 {
                    let segment = segments.first().unwrap();
                    if segment.ident.to_string() == "Option" {
                        match &segment.arguments {
                            PathArguments::AngleBracketed(argments) => {
                                let args = &argments.args;
                                if args.len() == 1 {
                                    let arg = args.first().unwrap();
                                    match arg {
                                        GenericArgument::Type(ty) => Some(ty),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        },
        _ => None,
    }
}

// Generated code looks like this:
// ```rust
// executable: Option<String>,
// current_dir: Option<String>,
// ```
fn builder_fields(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let mut ty = &f.ty;
                    if let Some(option_inner_ty) = extract_inner_if_option(ty) {
                        ty = option_inner_ty;
                    }
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
// pub fn current_dir(&mut self, current_dir: String) -> Self {
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
                    let mut ty = &f.ty;
                    if let Some(option_inner_ty) = extract_inner_if_option(ty) {
                        ty = option_inner_ty;
                    }
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
//         current_dir: self.args.take(),
//     })
// }
// ```
fn builder_build(name: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    match extract_inner_if_option(&f.ty) {
                        Some(_) => quote! {
                            #name: self.#name.take(),
                        },
                        None => {
                            let message =
                                format!("Missing field {}", name.as_ref().unwrap().to_string());
                            quote! {
                                #name: self.#name.take().ok_or(#message)?,
                            }
                        }
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
