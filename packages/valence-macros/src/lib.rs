mod helpers;

use helpers::{get_option_inner_type, has_skip_update_attr, merge_variants};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

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
                #[serde(skip_serializing_if = "Option::is_none")]
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

    // New: Generate fields for into_update implementation
    let into_update_fields = filtered_fields.iter().map(|f| {
        let name = &f.ident;

        if get_option_inner_type(&f.ty).is_some() {
            quote! {
                #name: OptionUpdate::Set(self.#name.clone()),
            }
        } else {
            quote! {
                #name: Some(self.#name.clone()),
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

            pub fn into_update(&self) -> #filter_name {
                #filter_name {
                    #(#into_update_fields)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn manager_impl_library_configs(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input_enum = parse_macro_input!(input as DeriveInput);
    let enum_ident = &input_enum.ident;
    // Create the new enum name by adding "Update" suffix
    let update_enum_ident = format_ident!("{}Update", enum_ident);

    // Extract variants from the original enum
    let variants = match &input_enum.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("This macro only works on enums"),
    };

    // Process each variant
    let mut update_variants = Vec::new();
    let mut update_msg_matches = Vec::new();
    let mut replace_config_matches = Vec::new();
    let mut get_instantiate_msg_matches = Vec::new();
    let mut pre_validate_matches = Vec::new();
    let mut get_account_ids_matches = Vec::new();

    for variant in variants {
        let variant_ident = &variant.ident;

        if variant_ident == "None" {
            // Add None variant
            update_variants.push(quote! {
                #[default]
                None
            });

            // Add None matches for all methods
            update_msg_matches.push(quote! {
                #update_enum_ident::None => return Err(LibraryError::NoLibraryConfigUpdate)
            });
            replace_config_matches.push(quote! {
                #enum_ident::None => return Err(LibraryError::NoLibraryConfig)
            });
            get_instantiate_msg_matches.push(quote! {
                #enum_ident::None => return Err(LibraryError::NoLibraryConfig)
            });
            pre_validate_matches.push(quote! {
                #enum_ident::None => Err(LibraryError::NoLibraryConfig)
            });
            get_account_ids_matches.push(quote! {
                #enum_ident::None => Err(LibraryError::NoLibraryConfig)
            });
            continue;
        }

        // Handle variants with inner types
        match &variant.fields {
            Fields::Unnamed(fields) => {
                let field = fields.unnamed.first().expect("Expected single field");
                if let Type::Path(type_path) = &field.ty {
                    let mut new_path = type_path.path.clone();
                    if let Some(last_seg) = new_path.segments.last_mut() {
                        last_seg.ident = format_ident!("{}Update", last_seg.ident);
                    }

                    // Extract the base module path
                    let module_path = &type_path.path.segments[0].ident;

                    // Add update variant
                    update_variants.push(quote! {
                        #variant_ident(#new_path)
                    });

                    // Add get_update_msg match for update enum
                    update_msg_matches.push(quote! {
                        #update_enum_ident::#variant_ident(service_config_update) => {
                            to_json_binary(&valence_library_utils::msg::ExecuteMsg::<
                                Empty,
                                #module_path::msg::LibraryConfigUpdate,
                            >::UpdateConfig {
                                new_config: service_config_update,
                            })
                        }
                    });

                    // Add replace_config match
                    replace_config_matches.push(quote! {
                        #enum_ident::#variant_ident(ref mut config) => {
                            let json = serde_json::to_string(&config)?;
                            let res = ac.replace_all(&json, &replace_with);
                            *config = serde_json::from_str(&res)?;
                        }
                    });

                    // Add get_instantiate_msg match
                    get_instantiate_msg_matches.push(quote! {
                        #enum_ident::#variant_ident(config) => to_vec(&InstantiateMsg {
                            owner,
                            processor,
                            config: config.clone(),
                        })
                    });

                    // Add pre_validate_config match
                    pre_validate_matches.push(quote! {
                        #enum_ident::#variant_ident(config) => {
                            config.pre_validate(api)?;
                            Ok(())
                        }
                    });

                    // Add get_account_ids match
                    get_account_ids_matches.push(quote! {
                        #enum_ident::#variant_ident(config) => {
                            Self::find_account_ids(ac, serde_json::to_string(&config)?)
                        }
                    });
                } else {
                    panic!("Expected Path type");
                }
            }
            _ => panic!("Expected unnamed fields"),
        }
    }

    // Generate the implementations
    let expanded = quote! {
        #[derive(
            Debug,
            Clone,
            strum::Display,
            Serialize,
            Deserialize,
            VariantNames,
            PartialEq,
            Default,
            JsonSchema,
        )]
        #[strum(serialize_all = "snake_case")]
        #[schemars(crate = "cosmwasm_schema::schemars")]
        pub enum #update_enum_ident {
            #(#update_variants,)*
        }

        impl #update_enum_ident {
            pub fn get_update_msg(self) -> LibraryResult<Binary> {
                match self {
                    #(#update_msg_matches,)*
                }
                .map_err(LibraryError::CosmwasmStdError)
            }
        }

        impl #enum_ident {
            pub fn replace_config(
                &mut self,
                patterns: Vec<String>,
                replace_with: Vec<String>,
            ) -> LibraryResult<()> {
                let ac = AhoCorasick::new(patterns)?;

                match self {
                    #(#replace_config_matches,)*
                }

                Ok(())
            }

            pub fn get_instantiate_msg(&self, owner: String, processor: String) -> LibraryResult<Vec<u8>> {
                match self {
                    #(#get_instantiate_msg_matches,)*
                }
                .map_err(LibraryError::SerdeJsonError)
            }

            pub fn pre_validate_config(&self, api: &dyn cosmwasm_std::Api) -> LibraryResult<()> {
                match self {
                    #(#pre_validate_matches,)*
                }
            }

            pub fn get_account_ids(&self) -> LibraryResult<Vec<Id>> {
                let ac: AhoCorasick = AhoCorasick::new(["\"|account_id|\":"]).unwrap();

                match self {
                    #(#get_account_ids_matches,)*
                }
            }
        }

        #input_enum
    };

    expanded.into()
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
