use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToUpperCamelCase,
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Attribute, FnArg, ItemTrait, LitStr, ReturnType, TraitItem, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn ts_callbacks(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemTrait);

    match expand_trait(item) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[derive(Default)]
struct SerdeNames {
    rename: Option<String>,
    rename_all: Option<String>,
}

fn parse_serde_attrs(attrs: &[Attribute]) -> syn::Result<SerdeNames> {
    let mut result = SerdeNames::default();

    for attr in attrs.iter().filter(|a| a.path().is_ident("serde")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: LitStr = meta.value()?.parse()?;
                result.rename = Some(value.value());
                Ok(())
            } else if meta.path.is_ident("rename_all") {
                let value: LitStr = meta.value()?.parse()?;
                result.rename_all = Some(value.value());
                Ok(())
            } else {
                Err(meta.error("supported here: `rename` and `rename_all`"))
            }
        })?;
    }

    Ok(result)
}

fn strip_serde_attrs(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("serde"));
}

fn rename_case(name: &str, rule: &str) -> syn::Result<String> {
    let renamed = match rule {
        "lowercase" => name.to_lowercase(),
        "UPPERCASE" => name.to_uppercase(),
        "PascalCase" => name.to_upper_camel_case(),
        "camelCase" => name.to_lower_camel_case(),
        "snake_case" => name.to_snake_case(),
        "SCREAMING_SNAKE_CASE" => name.to_shouty_snake_case(),
        "kebab-case" => name.to_kebab_case(),
        "SCREAMING-KEBAB-CASE" => name.to_shouty_kebab_case(),
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("unsupported serde rename_all rule `{rule}`"),
            ));
        }
    };

    Ok(renamed)
}

fn apply_name(
    rust_name: &str,
    names: &SerdeNames,
    inherited_rule: Option<&str>,
) -> syn::Result<String> {
    if let Some(rename) = &names.rename {
        return Ok(rename.clone());
    }

    if let Some(rule) = names.rename_all.as_deref().or(inherited_rule) {
        rename_case(rust_name, rule)
    } else {
        Ok(rust_name.to_owned())
    }
}

fn expand_trait(mut item: ItemTrait) -> syn::Result<proc_macro2::TokenStream> {
    let trait_names = parse_serde_attrs(&item.attrs)?;
    let trait_rule = trait_names.rename_all.as_deref();

    // `serde` helper attributes are consumed by this macro.
    strip_serde_attrs(&mut item.attrs);

    let trait_name = &item.ident;
    let section_ident = format_ident!("__TS_CALLBACKS_{}", trait_name);
    let mut declarations = Vec::new();

    for trait_item in &mut item.items {
        let TraitItem::Fn(method) = trait_item else {
            continue;
        };

        if !method.sig.generics.params.is_empty() || method.sig.generics.where_clause.is_some() {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "generic callback methods are not supported",
            ));
        }

        if method.sig.variadic.is_some() {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "variadic callback methods are not supported",
            ));
        }

        let method_names = parse_serde_attrs(&method.attrs)?;
        strip_serde_attrs(&mut method.attrs);

        let method_name = apply_name(&method.sig.ident.to_string(), &method_names, trait_rule)?;

        // Type aliases always use a valid, PascalCase-style identifier.
        let callback_name = format!("{}Callback", method_name.to_upper_camel_case());

        let parameter_rule = method_names.rename_all.as_deref().or(trait_rule);

        let mut parameters = Vec::new();

        for (index, arg) in method.sig.inputs.iter_mut().enumerate() {
            let FnArg::Typed(arg) = arg else {
                // A trait receiver (`&self`) isn't a JS callback argument.
                continue;
            };

            let arg_names = parse_serde_attrs(&arg.attrs)?;
            strip_serde_attrs(&mut arg.attrs);

            let rust_name = match arg.pat.as_ref() {
                syn::Pat::Ident(pat) => pat.ident.to_string(),
                _ => format!("arg{index}"),
            };

            let parameter_name = apply_name(&rust_name, &arg_names, parameter_rule)?;

            let parameter_type = ts_type(&arg.ty)?;

            parameters.push(format!("{parameter_name}: {parameter_type}"));
        }

        let return_type = match &method.sig.output {
            ReturnType::Default => "void".to_owned(),
            ReturnType::Type(_, ty) => {
                let value = ts_type(ty)?;

                if value == "void" {
                    "void".to_owned()
                } else {
                    value
                }
            }
        };

        declarations.push(format!(
            "export type {callback_name} = ({}) => {return_type};",
            parameters.join(", ")
        ));
    }

    let declarations = LitStr::new(&declarations.join("\n"), Span::call_site());

    Ok(quote! {
        #item

        #[::wasm_bindgen::prelude::wasm_bindgen(typescript_custom_section)]
        const #section_ident: &'static str = #declarations;
    })
}

fn ts_type(ty: &Type) -> syn::Result<String> {
    match ty {
        Type::Reference(reference) => ts_type(&reference.elem),

        Type::Array(array) => Ok(format!("{}[]", ts_type(&array.elem)?)),

        Type::Slice(slice) => Ok(format!("{}[]", ts_type(&slice.elem)?)),

        Type::Tuple(tuple) if tuple.elems.is_empty() => Ok("void".to_owned()),

        Type::Tuple(tuple) => {
            let elements = tuple
                .elems
                .iter()
                .map(ts_type)
                .collect::<syn::Result<Vec<_>>>()?;

            Ok(format!("[{}]", elements.join(", ")))
        }

        Type::Path(path) => {
            let segment = path
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(ty, "unsupported empty type path"))?;

            let name = segment.ident.to_string();

            match name.as_str() {
                "bool" => Ok("boolean".to_owned()),

                "String" | "str" | "char" => Ok("string".to_owned()),

                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64"
                | "i128" | "isize" | "f32" | "f64" => Ok("number".to_owned()),

                "JsValue" => Ok("unknown".to_owned()),

                "Uint8Array" | "Uint8ClampedArray" | "Int8Array" | "Uint16Array" | "Int16Array"
                | "Uint32Array" | "Int32Array" | "Float32Array" | "Float64Array"
                | "BigInt64Array" | "BigUint64Array" => Ok(name),

                "Promise" => {
                    let inner = generic_type(&segment.arguments)?;

                    Ok(format!(
                        "Promise<{}>",
                        inner.unwrap_or_else(|| "unknown".to_owned())
                    ))
                }

                "Option" => {
                    let inner = generic_type(&segment.arguments)?.ok_or_else(|| {
                        syn::Error::new_spanned(ty, "Option requires a type argument")
                    })?;

                    Ok(format!("{inner} | undefined"))
                }

                "Vec" => {
                    let inner = generic_type(&segment.arguments)?.ok_or_else(|| {
                        syn::Error::new_spanned(ty, "Vec requires a type argument")
                    })?;

                    Ok(format!("{inner}[]"))
                }

                _ => Err(syn::Error::new_spanned(
                    ty,
                    format!(
                        "no TypeScript mapping for `{name}`; \
                         add a mapping to ts_type()"
                    ),
                )),
            }
        }

        _ => Err(syn::Error::new_spanned(
            ty,
            "unsupported type in TypeScript callback signature",
        )),
    }
}

fn generic_type(arguments: &syn::PathArguments) -> syn::Result<Option<String>> {
    let syn::PathArguments::AngleBracketed(args) = arguments else {
        return Ok(None);
    };

    for argument in &args.args {
        if let syn::GenericArgument::Type(ty) = argument {
            return Ok(Some(ts_type(ty)?));
        }
    }

    Ok(None)
}
