mod helpers;

use helpers::{get_option_inner_type, has_skip_update_attr, merge_variants};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(ValenceLibraryInterface, attributes(skip_update))]
pub fn valence_library_interface_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let filter_name = format_ident!("{}Update", name);
    let vis = &ast.vis;

    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => panic!("ValenceLibraryUpdate only works on structs with named fields"),
        },
        _ => panic!("ValenceLibraryUpdate only works on structs"),
    };

    let filtered_fields: Vec<_> = fields
        .named
        .iter()
        .filter(|f| !has_skip_update_attr(&f.attrs))
        .collect();

    let filter_fields = filtered_fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        let vis = &f.vis;

        if let Some(inner_type) = get_option_inner_type(ty) {
            quote! {
                #vis #name: OptionUpdate<#inner_type>,
            }
        } else {
            quote! {
                #vis #name: Option<#ty>,
            }
        }
    });

    let update_fields = filtered_fields.iter().map(|f| {
        let name = &f.ident;

        if get_option_inner_type(&f.ty).is_some() {
            quote! {
                match &self.#name {
                    OptionUpdate::Set(value) => raw_config.#name = value.clone(),
                    OptionUpdate::None => {}
                }
            }
        } else {
            quote! {
                if let Some(value) = &self.#name {
                    raw_config.#name = value.clone();
                }
            }
        }
    });

    let diff_update_fields = filtered_fields.iter().map(|f| {
        let name = &f.ident;

        if get_option_inner_type(&f.ty).is_some() {
            quote! {
                if self.#name != other.#name {
                    update.#name = OptionUpdate::Set(other.#name.clone());
                }
            }
        } else {
            quote! {
                if self.#name != other.#name {
                    update.#name = Some(other.#name.clone());
                }
            }
        }
    });

    let expanded = quote! {
        use valence_library_utils::{LibraryConfigUpdateTrait, OptionUpdate, raw_config::{save_raw_library_config, load_raw_library_config}};
        use cosmwasm_std::StdResult;

        #[cw_serde]
        #[derive(Default)]
        #vis struct #filter_name {
            #(#filter_fields)*
        }

        impl LibraryConfigUpdateTrait for #filter_name {
            fn update_raw(&self, storage: &mut dyn cosmwasm_std::Storage) -> StdResult<()> {
                let mut raw_config = load_raw_library_config::<#name>(storage)?;

                #(#update_fields)*

                save_raw_library_config(storage, &raw_config)
            }
        }

        impl #name {
            pub fn get_diff_update(&self, other: #name) -> Option<#filter_name> {
                let mut update = #filter_name::default();
                let mut has_changes = false;

                #(#diff_update_fields)*

                if update != #filter_name::default() {
                    Some(update)
                } else {
                    None
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn valence_library_query(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum ValenceLibraryQuery {
                /// Query to get the processor address.
                #[returns(Addr)]
                GetProcessor {},
                /// Query to get the library configuration.
                #[returns(Config)]
                GetLibraryConfig {},
                #[returns(LibraryConfig)]
                GetRawLibraryConfig {},
            }
        )
        .into(),
    )
}
