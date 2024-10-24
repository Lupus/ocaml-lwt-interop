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
    let fn_body = &input.block;
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

    quote! {
        #(#other_attrs)*
        #ocaml_func_attr
        pub fn #fn_name(#fn_args) #fn_ret {
            let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
            let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                let res = #fn_body;
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
                let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
                let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                    let res = {
                        future::yield_now().await;
                        resolver.resolve(&ocaml_runtime(), &());
                        ()
                    };
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
                let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
                let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                    let res = {};
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
                let (fut, resolver) = ::ocaml_lwt_interop::promise::Promise::new(gc);
                let task = ::ocaml_lwt_interop::domain_executor::spawn_with_runtime(gc, async move {
                    let res = {};
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
