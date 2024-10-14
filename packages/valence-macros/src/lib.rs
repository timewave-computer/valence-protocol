mod helpers;

use helpers::{has_ignore_optional_attr, merge_variants};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(OptionalStruct, attributes(ignore_optional))]
pub fn optional_struct_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let filter_name = format_ident!("Optional{}", name);
    let vis = &ast.vis;

    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("OptionalStruct only works on structs with named fields"),
        },
        _ => panic!("OptionalStruct only works on structs"),
    };

    let filter_fields = fields.iter().filter_map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        let vis = &f.vis;
        let ignore = has_ignore_optional_attr(&f.attrs);
        if ignore {
            None
        } else {
            Some(quote! {
                #vis #name: std::option::Option<#ty>,
            })
        }
    });

    let update_fields = fields.iter().filter_map(|f| {
        let name = &f.ident;
        let ignore = has_ignore_optional_attr(&f.attrs);
        if ignore {
            None
        } else {
            Some(quote! {
                if let Some(#name) = self.#name.clone() {
                    raw_config.#name = #name;
                }
            })
        }
    });

    let expanded = quote! {
        use valence_service_utils::{OptionalServiceConfigTrait, raw_config::{save_raw_service_config, load_raw_service_config}};
        use cosmwasm_std::StdResult;

        #[cw_serde]
        #vis struct #filter_name {
            #(#filter_fields)*
        }

        impl OptionalServiceConfigTrait for #filter_name {
            fn update_raw(&self, storage: &mut dyn cosmwasm_std::Storage) -> StdResult<()> {
                let mut raw_config = load_raw_service_config::<#name>(storage)?;

                #(#update_fields)*

                save_raw_service_config(storage, &raw_config)
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn valence_service_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum ValenceServiceQuery {
                /// Query to get the processor address.
                #[returns(Addr)]
                GetProcessor {},
                /// Query to get the service configuration.
                #[returns(Config)]
                GetServiceConfig {},
                #[returns(ServiceConfig)]
                GetRawServiceConfig {},
            }
        )
        .into(),
    )
}
