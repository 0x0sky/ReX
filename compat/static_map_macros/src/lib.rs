// Compatibility shim for static_map_macros 0.2.0-beta.
//
// Upstream relied on the exact whitespace emitted by TokenStream::to_string()
// for the derive input. Modern rustc changed that formatting. The map builder
// itself remains unchanged; only extraction of the static_map! payload is made
// independent of rustc whitespace formatting.

extern crate fxhash;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

mod builder;
use builder::Builder;

type Key<'a> = syn::Lit;
type Value<'a> = &'a str;

fn trim(input: &str) -> &str {
    const MACRO: &str = "static_map!";

    let macro_start = input
        .find(MACRO)
        .expect("static_map! invocation missing from derive input");
    let after_macro = macro_start + MACRO.len();
    let open = input[after_macro..]
        .find('(')
        .map(|offset| after_macro + offset)
        .expect("static_map! invocation has no opening delimiter");

    let close = matching_paren(input, open)
        .expect("static_map! invocation has no closing delimiter");
    let body = input[open + 1..close].trim_start();

    assert!(body.starts_with('@'), "static_map! compatibility marker missing");
    let body = body[1..].trim_start();
    assert!(body.starts_with("zero"), "static_map! zero marker missing");

    body["zero".len()..].trim()
}

fn matching_paren(input: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    for (offset, ch) in input[open..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_string {
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        if in_char {
            match ch {
                '\\' => escaped = true,
                '\'' => in_char = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '\'' => in_char = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }

    None
}

#[proc_macro_derive(StaticMapMacro)]
pub fn static_map_macro(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let result = build_static_map(trim(&input));

    let wrapper = quote! {
        macro_rules! __static_map__construct_map {
            () => ( #result )
        }
    };

    wrapper.parse().unwrap()
}

fn build_static_map(input: &str) -> quote::Tokens {
    // Keep the original 0.2.0-beta payload format and builder semantics intact.
    let mut tokens = input.split('@');
    let default_value = tokens.next().unwrap();

    let count = input.chars().filter(|&c| c == '@').count();
    let mut builder = Builder::with_capacity(count);

    let mut pair = tokens
        .next()
        .expect("staticmap! requires at least one key/value pair")
        .split('?');

    let default_key = syn::parse::lit(pair.next().unwrap()).expect("failed to parse key type");
    let value = pair.next().unwrap();
    builder.insert(default_key.clone(), value);

    for pair in tokens {
        let mut pair = pair.split('?');
        let key = syn::parse::lit(pair.next().unwrap()).expect("failed to parse key type");
        let value = pair.next().unwrap();
        builder.insert(key, value);
    }

    builder.build(lit_default(&default_key), default_value)
}

fn lit_default(lit: &syn::Lit) -> syn::Lit {
    use syn::Lit::*;
    use syn::Lit;

    match *lit {
        Str(_, _) => Lit::from(""),
        Byte(_) => Lit::from(0u8),
        Char(_) => Lit::from(0 as char),
        Int(_, ty) => {
            use syn::IntTy::*;
            use syn::IntTy;
            match ty {
                Isize => Lit::from(0isize),
                I8 => Lit::from(0i8),
                I16 => Lit::from(0i16),
                I32 => Lit::from(0i32),
                I64 => Lit::from(0i64),
                Usize => Lit::from(0usize),
                U8 => Lit::from(0u8),
                U16 => Lit::from(0u16),
                U32 => Lit::from(0u32),
                U64 => Lit::from(0u64),
                Unsuffixed => Lit::Int(0, IntTy::Unsuffixed),
            }
        }
        ref lit => panic!("staticmap! unsupported key type `{:?}`", lit),
    }
}

#[cfg(test)]
mod tests {
    use super::trim;

    #[test]
    fn accepts_legacy_rustc_formatting() {
        let input = "enum __StaticMap__ {\n    A =\n        static_map!(@ zero DefaultValue @ 1u32 ? Value(2)),\n}";
        assert_eq!(trim(input), "DefaultValue @ 1u32 ? Value(2)");
    }

    #[test]
    fn accepts_modern_compact_formatting() {
        let input = "enum __StaticMap__ { A = static_map! (@ zero DefaultValue @ 1u32 ? Value(2)), }";
        assert_eq!(trim(input), "DefaultValue @ 1u32 ? Value(2)");
    }

    #[test]
    fn ignores_parentheses_inside_string_literals() {
        let input = "enum __StaticMap__ { A = static_map!(@zero DefaultValue @ \"(\" ? Value(2)), }";
        assert_eq!(trim(input), "DefaultValue @ \"(\" ? Value(2)");
    }
}
