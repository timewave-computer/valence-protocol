use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, FnArg, ItemTrait, Pat};

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

#[proc_macro_attribute]
pub fn connector_trait(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemTrait);
    let trait_name = &input.ident;

    let inner_trait_name = format_ident!("{}Inner", trait_name);

    let methods = input.items.iter().filter_map(|item| {
        if let syn::TraitItem::Method(method) = item {
            Some(method)
        } else {
            None
        }
    });

    let methods_without_new = methods.clone().filter(|m| m.sig.ident != "new");

    let method_impls = methods_without_new.clone().map(|method| {
        let method_name = &method.sig.ident;
        let inputs = &method.sig.inputs;
        let output = &method.sig.output;

        let args = inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Receiver(_) => None,
                FnArg::Typed(pat_type) => {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        Some(pat_ident.ident.clone())
                    } else {
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        quote! {
            fn #method_name(#inputs) #output {
                #trait_name::#method_name(self, #(#args),*)
            }
        }
    });

    let wrapper_methods = methods_without_new.clone().map(|method| {
        let method_name = &method.sig.ident;
        let inputs = &method.sig.inputs;
        let output = &method.sig.output;

        let args = inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Receiver(_) => None,
                FnArg::Typed(pat_type) => {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        Some(pat_ident.ident.clone())
                    } else {
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        quote! {
            fn #method_name(#inputs) #output {
                self.0.#method_name(#(#args),*)
            }
        }
    });

    let new_method = methods
        .clone()
        .find(|method| method.sig.ident == "new")
        .expect("new method not found");
    let new_inputs = &new_method.sig.inputs;
    let new_output = &new_method.sig.output;

    let new_args = new_inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    Some(pat_ident.ident.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let expanded = quote! {
        #input

        pub trait #inner_trait_name: Send + Sync + std::fmt::Debug {
            #(#methods_without_new)*
        }

        impl Clone for Box<dyn #inner_trait_name> {
            fn clone(&self) -> Self {
                self.to_owned()
            }
        }

        #[derive(Debug, Clone)]
        pub struct ConnectorWrapper(Box<dyn #inner_trait_name>);

        impl ConnectorWrapper {
            pub fn new<T>(#new_inputs) #new_output
            where
                T: #trait_name + #inner_trait_name + 'static,
            {
                Box::pin(async move {
                    let connector = T::new(#(#new_args),*).await;
                    ConnectorWrapper(Box::new(connector))
                })
            }
        }

        impl #inner_trait_name for ConnectorWrapper {
            #(#wrapper_methods)*
        }

        #[macro_export]
        macro_rules! impl_connector {
            ($t:ty) => {
                impl #inner_trait_name for $t {
                    #(#method_impls)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

// #[proc_macro_attribute]
// pub fn connector_trait(_attr: TokenStream, item: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(item as ItemTrait);
//     let trait_name = &input.ident;
//     let trait_methods = &input.items;

//     let inner_trait_methods = trait_methods.iter().filter_map(|item| match item {
//         TraitItem::Method(method) => {
//             if method.sig.ident != "new" {
//                 Some(quote! { #method })
//             } else {
//                 None
//             }
//         }
//         _ => None,
//     });

//     let inner_trait = quote! {
//         pub trait ConnectorInner: Send + Sync + std::fmt::Debug {
//             #(#inner_trait_methods)*
//         }
//     };

//     let wrapper_methods = trait_methods.iter().filter_map(|item| match item {
//         TraitItem::Method(method) => {
//             if method.sig.ident != "new" {
//                 let sig = &method.sig;
//                 let method_name = &sig.ident;
//                 let args = &sig.inputs;

//                 let arg_names = args.iter().filter_map(|arg| match arg {
//                     syn::FnArg::Typed(pat_type) => {
//                         if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
//                             Some(pat_ident.ident.clone())
//                         } else {
//                             None
//                         }
//                     }
//                     _ => None,
//                 });

//                 Some(quote! {
//                     #[allow(unused_variables)]
//                     #sig {
//                         self.0.#method_name(#(#arg_names),*)
//                     }
//                 })
//             } else {
//                 None
//             }
//         }
//         _ => None,
//     });

//     let wrapper_struct = quote! {
//         #[derive(Debug)]
//         pub struct ConnectorWrapper(Box<dyn ConnectorInner>);

//         impl ConnectorWrapper {
//             pub async fn new<T>(endpoint: String, wallet_mnemonic: String) -> Self
//             where
//                 T: #trait_name + ConnectorInner + 'static,
//             {
//                 let connector = T::new(endpoint, wallet_mnemonic).await;
//                 ConnectorWrapper(Box::new(connector))
//             }
//         }

//         impl ConnectorInner for ConnectorWrapper {
//             #(#wrapper_methods)*
//         }
//     };

//     let expanded = quote! {
//         #input

//         #inner_trait

//         #wrapper_struct
//     };

//     expanded.into()
// }

// #[proc_macro_attribute]
// pub fn impl_connector_inner(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(item as DeriveInput);
//     let attr_input = parse_macro_input!(attr as Path);
//     let struct_name = &input.ident;

//     let (methods, trait_ident) = get_connector_trait_methods(attr_input);

//     let method_impls = methods.iter().map(|method| {
//         let method_name = &method.sig.ident;
//         let args = &method.sig.inputs;
//         let output = &method.sig.output;

//         let arg_names = args.iter().filter_map(|arg| {
//             if let syn::FnArg::Typed(pat_type) = arg {
//                 if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
//                     Some(pat_ident.ident.clone())
//                 } else {
//                     None
//                 }
//             } else {
//                 None
//             }
//         });

//         quote! {
//             #[allow(unused_variables)]
//             fn #method_name(#args) #output {
//                 <Self as #trait_ident>::#method_name(self, #(#arg_names),*)
//             }
//         }
//     }).collect::<Vec<_>>();

//     let expanded = quote! {
//         impl ConnectorInner for #struct_name {
//             #(#method_impls)*
//         }
//     };

//     expanded.into()
// }

// fn get_connector_trait_methods(trait_path: &Path) -> (Vec<TraitItemMethod>, Ident) {
//     let trait_ident = trait_path.segments.last().unwrap().ident.clone();

//     if let Item::Trait(trait_item) = trait_ident {
//         (trait_item.items.into_iter().filter_map(|item| {
//             if let TraitItem::Method(method) = item {
//                 Some(method)
//             } else {
//                 None
//             }
//         }).collect(), trait_item.ident)
//     } else {
//         panic!("Expected a trait")
//     }
// }
