extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::quote;
use syn::Fields;

#[proc_macro_derive(CairoSerde)]
pub fn cairo_serde_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    let result = impl_cairo_serde(&ast);

    TokenStream::from(result)
}

fn impl_cairo_serde(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;

    let all_fields = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields.named.iter().collect::<Vec<_>>(),
            Fields::Unnamed(fields) => fields.unnamed.iter().collect::<Vec<_>>(),
            Fields::Unit => vec![],
        },
        _ => panic!("Only structs are supported"),
    };

    let serialize_fields = all_fields
        .iter()
        .enumerate()
        .map(|(index, field)| match &field.ident {
            Some(ident) => quote! {
                result.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&self.#ident));
            },
            None => {
                let index = Literal::usize_unsuffixed(index);
                quote! {
                    result.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&self.#index));
                }
            }
        })
        .collect::<Vec<proc_macro2::TokenStream>>();

    quote! {
        impl ::cairo_serde::traits::CairoSerializable for #name {
            fn serialize_cairo(&self) -> Vec<::cairo_serde::traits::UniversalFelt> {
                let mut result = Vec::new();
                #(#serialize_fields)*
                result
            }
        }
    }
}

// test if the macro works
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impl_cairo_serde() {
        let input = r#"
            struct Test {
                a: u32,
                b: String,
            }
        "#;

        let ast = syn::parse_str(input).unwrap();
        let result = impl_cairo_serde(&ast);

        let expected = quote! {
            impl ::cairo_serde::traits::CairoSerializable for Test {
                fn serialize_cairo(&self) -> Vec<::cairo_serde::traits::UniversalFelt> {
                    let mut result = Vec::new();
                    result.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&self.a));
                    result.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&self.b));
                    result
                }
            }
        };

        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_impl_cairo_serde_with_unit_struct() {
        let input = r#"
            struct Test;
        "#;

        let ast = syn::parse_str(input).unwrap();
        let result = impl_cairo_serde(&ast);

        let expected = quote! {
            impl ::cairo_serde::traits::CairoSerializable for Test {
                fn serialize_cairo(&self) -> Vec<::cairo_serde::traits::UniversalFelt> {
                    let mut result = Vec::new();
                    result
                }
            }
        };

        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_impl_cairo_serde_with_unnamed_fields() {
        let input = r#"
            struct Test(u32, String);
        "#;

        let ast = syn::parse_str(input).unwrap();
        let result = impl_cairo_serde(&ast);

        let expected = quote! {
            impl ::cairo_serde::traits::CairoSerializable for Test {
                fn serialize_cairo(&self) -> Vec<::cairo_serde::traits::UniversalFelt> {
                    let mut result = Vec::new();
                    result.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&self.0));
                    result.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&self.1));
                    result
                }
            }
        };

        assert_eq!(result.to_string(), expected.to_string());
    }
}
