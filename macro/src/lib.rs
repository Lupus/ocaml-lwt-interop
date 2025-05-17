use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn func(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    func_impl(input).into()
}

fn paths_equal(path1: &syn::Path, path2: &syn::Path) -> bool {
    if path1.leading_colon.is_some() != path2.leading_colon.is_some() {
        return false;
    }

    if path1.segments.len() != path2.segments.len() {
        return false;
    }

    path1
        .segments
        .iter()
        .zip(path2.segments.iter())
        .all(|(seg1, seg2)| segments_equal(seg1, seg2))
}

fn segments_equal(seg1: &syn::PathSegment, seg2: &syn::PathSegment) -> bool {
    if seg1.ident != seg2.ident {
        return false;
    }

    match (&seg1.arguments, &seg2.arguments) {
        (syn::PathArguments::None, syn::PathArguments::None) => true,
        (
            syn::PathArguments::AngleBracketed(_),
            syn::PathArguments::AngleBracketed(_),
        ) => {
            unimplemented!("Path equality with angle bracket args is not implemented")
        }
        (syn::PathArguments::Parenthesized(_), syn::PathArguments::Parenthesized(_)) => {
            unimplemented!("Path equality with parenthesized args is not implemented")
        }
        _ => false,
    }
}

fn func_impl(input: ItemFn) -> TokenStream2 {
    let fn_name = &input.sig.ident;
    let fn_body_stmts = &input.block.stmts;
    let fn_args = &input.sig.inputs;
    let fn_ret = match &input.sig.output {
        syn::ReturnType::Default => syn::parse2::<syn::ReturnType>(
            quote! { -> ::ocaml_lwt_interop::promise::Promise<()> },
        )
        .unwrap(),
        syn::ReturnType::Type(rarrow, typ) => {
            let new_typ = syn::parse2::<syn::Type>(
                quote! { ::ocaml_lwt_interop::promise::Promise<#typ> },
            )
            .unwrap();
            syn::ReturnType::Type(*rarrow, Box::new(new_typ))
        }
    };

    let expected_path = syn::parse2::<syn::Path>(quote! {ocaml::func}).unwrap();
    // Separate #[ocaml::func] from other attributes
    let (ocaml_func_attr, other_attrs): (Vec<_>, Vec<_>) = input
        .attrs
        .into_iter()
        .partition(|attr| paths_equal(&attr.path, &expected_path));

    // Use the existing #[ocaml::func] if present, otherwise create a default one
    let ocaml_func_attr = if !ocaml_func_attr.is_empty() {
        quote! { #(#ocaml_func_attr)* }
    } else {
        quote! { #[ocaml::func] }
    };

    // Prepare arguments for calling the inner function. We expect patterns to be
    // simple identifiers.
    let call_args: Vec<syn::Ident> = fn_args
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => match pat_type.pat.as_ref() {
                syn::Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let inner_fn_name = syn::Ident::new("__inner", proc_macro2::Span::call_site());
    let fn_generics = &input.sig.generics;
    let fn_output = match &input.sig.output {
        syn::ReturnType::Default => {
            syn::parse2::<syn::ReturnType>(quote! { -> () }).unwrap()
        }
        other => other.clone(),
    };

    quote! {
        #(#other_attrs)*
        #ocaml_func_attr
        pub fn #fn_name(#fn_args) #fn_ret {
            async fn #inner_fn_name #fn_generics(#fn_args) #fn_output {
                #(#fn_body_stmts)*
            }
            let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
            let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                let res = #inner_fn_name(#(#call_args),*).await;
                let gc = &::ocaml_lwt_interop::domain_executor::ocaml_runtime();
                resolver.resolve(gc, &res);
            });
            task.detach();
            fut
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream as TokenStream2;

    fn assert_tokens_eq(actual: TokenStream2, expected: TokenStream2) {
        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn test_ocaml_lwt_interop_func() {
        let input: TokenStream2 = quote! {
            pub fn lwti_tests_bench() -> () {
                future::yield_now().await;
                resolver.resolve(&ocaml_runtime(), &());
                ()
            }
        };

        let expected: TokenStream2 = quote! {
            #[ocaml::func]
            pub fn lwti_tests_bench() -> ::ocaml_lwt_interop::promise::Promise<()> {
                async fn __inner() -> () {
                    future::yield_now().await;
                    resolver.resolve(&ocaml_runtime(), &());
                    ()
                }
                let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
                let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                    let res = __inner().await;
                    let gc = &::ocaml_lwt_interop::domain_executor::ocaml_runtime();
                    resolver.resolve(gc, &res);
                });
                task.detach();
                fut
            }
        };

        let input_fn = syn::parse2::<ItemFn>(input).unwrap();
        let actual = func_impl(input_fn);
        assert_tokens_eq(actual, expected);
    }

    #[test]
    fn test_ocaml_lwt_interop_func_with_existing_ocaml_func() {
        let input: TokenStream2 = quote! {
            #[ocaml::func(whatever)]
            pub fn lwti_tests_bench() {}
        };

        let expected: TokenStream2 = quote! {
            #[ocaml::func(whatever)]
            pub fn lwti_tests_bench() -> ::ocaml_lwt_interop::promise::Promise<()> {
                async fn __inner() -> () {
                }
                let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
                let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                    let res = __inner().await;
                    let gc = &::ocaml_lwt_interop::domain_executor::ocaml_runtime();
                    resolver.resolve(gc, &res);
                });
                task.detach();
                fut
            }
        };

        let input_fn = syn::parse2::<ItemFn>(input).unwrap();
        let actual = func_impl(input_fn);
        assert_tokens_eq(actual, expected);
    }

    #[test]
    fn test_ocaml_lwt_interop_func_with_args() {
        let input: TokenStream2 = quote! {
            pub fn lwti_tests_bench(arg1: String, args2: u32) -> u64 {}
        };

        let expected: TokenStream2 = quote! {
            #[ocaml::func]
            pub fn lwti_tests_bench(arg1: String, args2: u32) -> ::ocaml_lwt_interop::promise::Promise<u64> {
                async fn __inner(arg1: String, args2: u32) -> u64 {
                }
                let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
                let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                    let res = __inner(arg1, args2).await;
                    let gc = &::ocaml_lwt_interop::domain_executor::ocaml_runtime();
                    resolver.resolve(gc, &res);
                });
                task.detach();
                fut
            }
        };

        let input_fn = syn::parse2::<ItemFn>(input).unwrap();
        let actual = func_impl(input_fn);
        assert_tokens_eq(actual, expected);
    }
}
