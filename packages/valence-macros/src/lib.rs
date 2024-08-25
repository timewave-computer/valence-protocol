use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(OptionalStruct)]
pub fn optional_struct_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let filter_name = syn::Ident::new(&format!("Optional{}", name), name.span());
    let vis = &ast.vis;

    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("OptionalStruct only works on structs with named fields"),
        },
        _ => panic!("OptionalStruct only works on structs"),
    };

    let filter_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        let vis = &f.vis;

        quote! {
            #vis #name: std::option::Option<#ty>
        }
    });

    let expanded = quote! {
        #[cw_serde]
        #vis struct #filter_name {
            #(#filter_fields,)*
        }
    };

    TokenStream::from(expanded)
}
