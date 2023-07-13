use std::collections::HashMap;

use inflector::Inflector;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, ExprArray, ExprPath, Ident, Token,
};

fn expr_path_to_ident(expr: &ExprPath) -> Ident {
    expr.path.get_ident().unwrap().clone()
}

fn expr_to_ident(expr: &Expr) -> Ident {
    match expr {
        Expr::Path(expr) => expr_path_to_ident(expr),
        _ => panic!("Expected Ident"),
    }
}

fn parse_idents_array(input: ParseStream) -> syn::Result<Vec<Ident>> {
    Ok(input
        .parse::<ExprArray>()?
        .elems
        .iter()
        .map(expr_to_ident)
        .collect())
}

struct FixedMapArgs {
    name: Ident,
    variants: Vec<ExprPath>,
}

impl Parse for FixedMapArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<Ident> = Default::default();
        let mut variants: Option<Vec<ExprPath>> = Default::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=>]>()?;
            match &*key.to_string() {
                "name" => {
                    name = Some(input.parse::<Ident>()?);
                }
                "variants" => {
                    variants = Some(
                        input
                            .parse::<ExprArray>()?
                            .elems
                            .into_iter()
                            .map(|expr| match expr {
                                Expr::Path(expr) => expr,
                                _ => panic!("Expected Ident, maybe with attributes"),
                            })
                            .collect(),
                    );
                }
                _ => panic!("didn't expect key {}", key),
            }
            input.parse::<Token![,]>()?;
        }
        Ok(FixedMapArgs {
            name: name.expect("Expected name specifier"),
            variants: variants.expect("Expected variants specifier"),
        })
    }
}

#[proc_macro]
pub fn fixed_map(input: TokenStream) -> TokenStream {
    let FixedMapArgs {
        name,
        variants: variants_with_attributes,
    } = parse_macro_input!(input as FixedMapArgs);
    let variants = variants_with_attributes
        .iter()
        .map(expr_path_to_ident)
        .collect::<Vec<_>>();

    let collection_type_name = format_ident!("By{name}");
    let all_variants_collection_name = format_ident!(
        "ALL_{}",
        name.to_string().to_plural().to_screaming_snake_case()
    );

    let token_enum_definition = get_token_enum_definition(&name, &variants_with_attributes);
    let collection_type_definition =
        get_collection_type_definition(&collection_type_name, &variants);
    let iter_implementations =
        get_iter_implementations(&name, &collection_type_name, &all_variants_collection_name);
    let index_implementations = get_index_implementations(&name, &collection_type_name);
    let deref_implementation = get_deref_implementation(&collection_type_name, &variants);
    let from_usize_implementation = get_from_usize_implementation(&name, &variants);
    let all_variants_collection_definition = get_all_variants_collection_definition(
        &name,
        &collection_type_name,
        &variants,
        &all_variants_collection_name,
    );
    let collection_generator_macro_definition =
        get_collection_generator_macro_definition(&collection_type_name, &variants);

    quote! {
        #token_enum_definition

        #collection_type_definition

        #iter_implementations

        #index_implementations

        #deref_implementation

        #from_usize_implementation

        #all_variants_collection_definition

        #collection_generator_macro_definition
    }
    .into()
}

fn get_token_enum_definition(
    name: &Ident,
    variants_with_attributes: &[ExprPath],
) -> proc_macro2::TokenStream {
    quote! {
        #[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum, strum_macros::Display)]
        pub enum #name {
            #(#variants_with_attributes),*
        }
    }
}

fn get_collection_type_definition(
    collection_type_name: &Ident,
    variants: &[Ident],
) -> proc_macro2::TokenStream {
    let len = variants.len();
    quote! {
        #[derive(Default)]
        pub struct #collection_type_name<T>([T; #len]);
    }
}

fn get_iter_implementations(
    name: &Ident,
    collection_type_name: &Ident,
    all_variants_collection_name: &Ident,
) -> proc_macro2::TokenStream {
    let iter_struct_name = format_ident!("{collection_type_name}Iter");
    let values_struct_name = format_ident!("{collection_type_name}Values");

    quote! {
        impl<T> #collection_type_name<T> {
            pub fn iter(&self) -> #iter_struct_name<'_, T> {
                #iter_struct_name::new(self)
            }

            pub fn values(&self) -> #values_struct_name<'_, T> {
                #values_struct_name::new(self)
            }
        }

        pub struct #iter_struct_name<'collection, T> {
            collection: &'collection #collection_type_name<T>,
            next_index: usize,
        }

        impl<'collection, T> #iter_struct_name<'collection, T> {
            pub fn new(collection: &'collection #collection_type_name<T>) -> Self {
                Self {
                    collection,
                    next_index: 0,
                }
            }
        }

        impl<'collection, T> Iterator for #iter_struct_name<'collection, T> {
            type Item = (#name, &'collection T);

            fn next(&mut self) -> Option<Self::Item> {
                if self.next_index < self.collection.len() {
                    let ret = Some((
                        #all_variants_collection_name[self.next_index],
                        &self.collection.0[self.next_index],
                    ));
                    self.next_index += 1;
                    ret
                } else {
                    None
                }
            }
        }

        pub struct #values_struct_name<'collection, T> {
            collection: &'collection #collection_type_name<T>,
            next_index: usize,
        }

        impl<'collection, T> #values_struct_name<'collection, T> {
            pub fn new(collection: &'collection #collection_type_name<T>) -> Self {
                Self {
                    collection,
                    next_index: 0,
                }
            }
        }

        impl<'collection, T> Iterator for #values_struct_name<'collection, T> {
            type Item = &'collection T;

            fn next(&mut self) -> Option<Self::Item> {
                if self.next_index < self.collection.len() {
                    let ret = Some(&self.collection.0[self.next_index]);
                    self.next_index += 1;
                    ret
                } else {
                    None
                }
            }
        }
    }
}

fn get_index_implementations(
    name: &Ident,
    collection_type_name: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        impl<T> Index<#name> for #collection_type_name<T> {
            type Output = T;

            fn index(&self, index: #name) -> &Self::Output {
                &self.0[index as usize]
            }
        }

        impl<T> Index<usize> for #collection_type_name<T> {
            type Output = T;

            fn index(&self, index: usize) -> &Self::Output {
                &self.0[index]
            }
        }
    }
}

fn get_deref_implementation(
    collection_type_name: &Ident,
    variants: &[Ident],
) -> proc_macro2::TokenStream {
    let len = variants.len();
    quote! {
        impl<T> Deref for #collection_type_name<T> {
            type Target = [T; #len];

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    }
}

fn get_from_usize_implementation(name: &Ident, variants: &[Ident]) -> proc_macro2::TokenStream {
    quote! {
        impl From<usize> for #name {
            fn from(value: usize) -> Self {
                match value {
                    #(value if value == Self::#variants as usize => Self::#variants),*,
                    _ => unreachable!(),
                }
            }
        }
    }
}

fn get_all_variants_collection_definition(
    name: &Ident,
    collection_type_name: &Ident,
    variants: &[Ident],
    all_variants_collection_name: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        pub static #all_variants_collection_name: #collection_type_name<#name> = {
            use SupportedLanguage::*;
            BySupportedLanguage([
                #(#variants),*
            ])
        };
    }
}

fn get_collection_generator_macro_definition(
    collection_type_name: &Ident,
    variants: &[Ident],
) -> proc_macro2::TokenStream {
    let macro_name = format_ident!("{}", collection_type_name.to_string().to_snake_case());
    quote! {
        #[macro_export]
        macro_rules! #macro_name {
            ($($variant:ident => $value:expr),* $(,)?) => {
                proc_macros::by_fixed_map!(
                    [$($variant => $value),*],
                    [
                        #(#variants),*
                    ],
                    #collection_type_name
                )
            }
        }
    }
}

struct ByFixedMapArgs {
    value_mapping: HashMap<Ident, Expr>,
    variants: Vec<Ident>,
    collection_type_name: Ident,
}

impl Parse for ByFixedMapArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let value_mapping_content;
        bracketed!(value_mapping_content in input);
        let mut value_mapping: HashMap<Ident, Expr> = Default::default();
        while !value_mapping_content.is_empty() {
            let key: Ident = value_mapping_content.parse()?;
            value_mapping_content.parse::<Token![=>]>()?;
            let value: Expr = value_mapping_content.parse()?;
            value_mapping.insert(key, value);
            if value_mapping_content.is_empty() {
                break;
            }
            value_mapping_content.parse::<Token![,]>()?;
        }
        input.parse::<Token![,]>()?;
        let variants = parse_idents_array(input)?;
        input.parse::<Token![,]>()?;
        let collection_type_name: Ident = input.parse()?;
        Ok(Self {
            value_mapping,
            variants,
            collection_type_name,
        })
    }
}

#[proc_macro]
pub fn by_fixed_map(input: TokenStream) -> TokenStream {
    let ByFixedMapArgs {
        value_mapping,
        variants,
        collection_type_name,
    } = parse_macro_input!(input as ByFixedMapArgs);

    if value_mapping.len() != variants.len() {
        panic!("Incorrect variants");
    }

    let ordered_values = variants
        .iter()
        .map(|variant| value_mapping.get(variant).expect("Incorrect variants"))
        .collect::<Vec<_>>();

    quote! {
        #collection_type_name([
            #(#ordered_values),*
        ])
    }
    .into()
}
