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
    
    let inner_methods = methods.clone().filter(|m| m.sig.ident != "new");

    let method_impls = methods.clone().filter(|m| m.sig.ident != "new").map(|method| {
        let method_name = &method.sig.ident;
        let inputs = &method.sig.inputs;
        let output = &method.sig.output;
        
        let args = inputs.iter().filter_map(|arg| {
            match arg {
                FnArg::Receiver(_) => None,
                FnArg::Typed(pat_type) => {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        Some(pat_ident.ident.clone())
                    } else {
                        None
                    }
                }
            }
        }).collect::<Vec<_>>();
        
        quote! {
            fn #method_name(#inputs) #output {
                #trait_name::#method_name(self, #(#args),*)
            }
        }
    });

    let wrapper_methods = methods.clone().filter(|m| m.sig.ident != "new").map(|method| {
        let method_name = &method.sig.ident;
        let inputs = &method.sig.inputs;
        let output = &method.sig.output;
        
        let args = inputs.iter().filter_map(|arg| {
            match arg {
                FnArg::Receiver(_) => None,
                FnArg::Typed(pat_type) => {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        Some(pat_ident.ident.clone())
                    } else {
                        None
                    }
                }
            }
        }).collect::<Vec<_>>();
        
        quote! {
            fn #method_name(#inputs) #output {
                self.0.#method_name(#(#args),*)
            }
        }
    });

    let new_method = methods.clone().find(|method| method.sig.ident == "new").expect("new method not found");
    let new_inputs = &new_method.sig.inputs;
    let new_output = &new_method.sig.output;

    let new_args = new_inputs.iter().filter_map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                Some(pat_ident.ident.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).collect::<Vec<_>>();

    let expanded = quote! {
        #input
        
        pub trait #inner_trait_name: Send + Sync + std::fmt::Debug {
            #(#inner_methods)*
        }
        
        #[derive(Debug)]
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
        
        #[doc(hidden)]
        #[macro_export]
        macro_rules! impl_connector {
            ($t:ty) => {
                impl #inner_trait_name for $t {
                    #(#method_impls)*
                }
            }
        }

        pub use impl_connector as __impl_connector;
    };
    
    TokenStream::from(expanded)
}
