use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemTrait, TraitItem, TraitItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn unimplemented_impl(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemTrait);

    match expand(item) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand(item: ItemTrait) -> syn::Result<proc_macro2::TokenStream> {
    let trait_name = &item.ident;

    let impl_name = syn::Ident::new(&format!("{}Unimplemented", trait_name), trait_name.span());

    let methods = item.items.iter().filter_map(|item| {
        let TraitItem::Fn(method) = item else {
            return None;
        };

        // Methods with default implementations don't need overriding.
        if method.default.is_some() {
            return None;
        }

        Some(method)
    });

    let implementations = methods
        .map(generate_method)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #item

        pub struct #impl_name;

        impl #trait_name for #impl_name {
            #(#implementations)*
        }
    })
}

fn generate_method(method: &TraitItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let signature = &method.sig;

    if signature.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            signature,
            "async trait methods are not supported by this macro yet",
        ));
    }

    if signature.constness.is_some() || signature.variadic.is_some() {
        return Err(syn::Error::new_spanned(
            signature,
            "const and variadic trait methods are unsupported",
        ));
    }

    Ok(quote! {
        #signature {
            ::core::unimplemented!()
        }
    })
}
