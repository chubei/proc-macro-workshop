use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DeriveInput, Error, Field, Fields, GenericArgument,
    Ident, Lit, Meta, NestedMeta, PathArguments, Result, Type,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let vis = input.vis;

    // Generate builder struct name.
    let builder_name = format_ident!("{}Builder", name);

    // Inspect all struct fields.
    let fields = convert_fields(&input.data);
    if let Err(err) = fields {
        return proc_macro::TokenStream::from(err.to_compile_error());
    }
    let fields = fields.unwrap();

    // Generate builder fields.
    let builder_fields = fields.iter().map(|f| f.field_token());
    // Generate setters for all the fields.
    let builder_setters = fields.iter().map(|f| f.setter_token());
    // Generate struct constructor.
    let builder_constructor = fields.iter().map(|f| f.build_token());

    let expanded = quote! {
        #[derive(Default)]
        #vis struct #builder_name {
            #(#builder_fields)*
        }

        impl #builder_name {
            #(#builder_setters)*

            pub fn build(&mut self) -> Result<#name, &'static str> {
                Ok(#name {
                    #(#builder_constructor)*
                })
            }
        }

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name::default()
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

struct OptionalBuilderField<'a> {
    pub field: &'a Field,
    pub ty: &'a Type,
}

struct RepeatedBuilderField<'a> {
    pub field: &'a Field,
    pub ty: &'a Type,
    pub each_name: Ident,
}

enum BuilderField<'a> {
    AllAtOnce(&'a Field),
    Optional(OptionalBuilderField<'a>),
    Repeated(RepeatedBuilderField<'a>),
}

impl<'a> BuilderField<'a> {
    // Generated code looks like this:
    // ```rust
    // executable: Option<String>,
    // current_dir: Option<String>,
    // args: Vec<String>,
    // ```
    pub fn field_token(&self) -> TokenStream {
        match self {
            Self::AllAtOnce(field) => {
                let name = &field.ident;
                let ty = &field.ty;
                quote! {
                    #name: Option<#ty>,
                }
            }
            Self::Optional(field) => {
                let name = &field.field.ident;
                let ty = &field.ty;
                quote! {
                    #name: Option<#ty>,
                }
            }
            Self::Repeated(field) => {
                let name = &field.field.ident;
                let ty = &field.ty;
                quote! {
                    #name: Vec<#ty>,
                }
            }
        }
    }

    // Generated code looks like this:
    // ```rust
    // pub fn executable(&mut self, executable: String) -> &mut Self {
    //     self.executable = Some(executable);
    //     self
    // }
    // pub fn current_dir(&mut self, current_dir: String) -> &mut Self {
    //     self.current_dir = Some(current_dir);
    //     self
    // }
    // pub fn arg(&mut self, arg: String) -> &mut Self {
    //     self.args.push(arg);
    //     self
    // }
    // pub fn args(&mut self, args: Vec<String>) -> &mut Self {
    //     self.args = args;
    //     self
    // }
    // ```
    pub fn setter_token(&self) -> TokenStream {
        match self {
            Self::AllAtOnce(field) => {
                let name = &field.ident;
                let ty = &field.ty;
                quote! {
                    pub fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name = Some(#name);
                        self
                    }
                }
            }
            Self::Optional(field) => {
                let name = &field.field.ident;
                let ty = &field.ty;
                quote! {
                    pub fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name = Some(#name);
                        self
                    }
                }
            }
            Self::Repeated(field) => {
                let field_name = field.field.ident.as_ref();
                let each_name = &field.each_name;
                let once_setter = if field_name.unwrap() == each_name {
                    quote! {}
                } else {
                    let filed_ty = &field.field.ty;
                    quote! {
                        pub fn #field_name(&mut self, #field_name: #filed_ty) -> &mut Self {
                            self.#field_name = #field_name;
                            self
                        }
                    }
                };

                let ty = &field.ty;
                quote! {
                    #once_setter

                    pub fn #each_name(&mut self, #each_name: #ty) -> &mut Self {
                        self.#field_name.push(#each_name);
                        self
                    }
                }
            }
        }
    }

    // Generated code looks like this:
    // ```rust
    // executable: self.executable.take().ok_or("Missing field executable")?,
    // current_dir: self.args.take(),
    // args: std::mem::replace(&mut self.args, vec![]),
    // ```
    pub fn build_token(&self) -> TokenStream {
        match self {
            Self::AllAtOnce(field) => {
                let name = field.ident.as_ref();
                let message = format!("Missing required field {}", name.unwrap().to_string());
                quote! {
                    #name: self.#name.take().ok_or(#message)?,
                }
            }
            Self::Optional(field) => {
                let name = &field.field.ident;
                quote! {
                    #name: self.#name.take(),
                }
            }
            Self::Repeated(field) => {
                let name = &field.field.ident;
                quote! {
                    #name: std::mem::replace(&mut self.#name, vec![]),
                }
            }
        }
    }
}

// Convert all the fields in `data`.
fn convert_fields<'a>(data: &'a Data) -> Result<Vec<BuilderField<'a>>> {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields.named.iter().map(|f| convert_field(f)).collect(),
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

// Convert a `field`.
fn convert_field<'a>(field: &'a Field) -> Result<BuilderField<'a>> {
    let mut each_name = Option::<Ident>::default();
    for attr in &field.attrs {
        if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
            if meta_list.path.is_ident("builder") {
                for nested in &meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                        if name_value.path.is_ident("each") {
                            if let Lit::Str(name) = &name_value.lit {
                                each_name = Some(Ident::new(&name.value(), name.span()));
                                break;
                            }
                        } else {
                            let span = attr.path.span().join(attr.tokens.span()).unwrap();
                            return Err(Error::new(span, r#"expected `builder(each = "...")`"#));
                        }
                    }
                }
            }
        }
    }

    if let Type::Path(path) = &field.ty {
        if let None = &path.qself {
            let segments = &path.path.segments;
            if segments.len() == 1 {
                let segment = segments.first().unwrap();
                if segment.ident == "Option" || segment.ident == "Vec" {
                    if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
                        let args = &arguments.args;
                        if args.len() == 1 {
                            let arg = args.first().unwrap();
                            if let GenericArgument::Type(ty) = arg {
                                if segment.ident == "Option" {
                                    return Ok(BuilderField::Optional(OptionalBuilderField {
                                        field,
                                        ty,
                                    }));
                                } else if let Some(each_name) = each_name {
                                    return Ok(BuilderField::Repeated(RepeatedBuilderField {
                                        field,
                                        each_name,
                                        ty,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return Ok(BuilderField::AllAtOnce(field));
}
