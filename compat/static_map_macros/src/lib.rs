use phf_generator::HashState;
use phf_shared::PhfHash;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::collections::HashSet;
use std::hash::Hasher;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Error, Expr, ExprLit, Lit, LitInt, Result, Token};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum ParsedKey {
    Str(String),
    U32(u32),
    Char(char),
    Bool(bool),
}

impl PhfHash for ParsedKey {
    fn phf_hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ParsedKey::Str(value) => value.phf_hash(state),
            ParsedKey::U32(value) => value.phf_hash(state),
            ParsedKey::Char(value) => value.phf_hash(state),
            ParsedKey::Bool(value) => value.phf_hash(state),
        }
    }
}

struct Key {
    parsed: ParsedKey,
    tokens: TokenStream2,
}

impl Key {
    fn from_expr(expr: Expr) -> Result<Self> {
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) => Ok(Key {
                parsed: ParsedKey::Str(value.value()),
                tokens: value.to_token_stream(),
            }),
            Expr::Lit(ExprLit {
                lit: Lit::Int(value),
                ..
            }) => {
                let suffix = value.suffix();
                if !suffix.is_empty() && suffix != "u32" {
                    return Err(Error::new_spanned(
                        value,
                        "ReX static_map compatibility supports only u32 integer keys",
                    ));
                }

                let parsed = value.base10_parse::<u32>()?;
                let typed = LitInt::new(&format!("{}u32", parsed), value.span());
                Ok(Key {
                    parsed: ParsedKey::U32(parsed),
                    tokens: typed.to_token_stream(),
                })
            }
            Expr::Lit(ExprLit {
                lit: Lit::Char(value),
                ..
            }) => Ok(Key {
                parsed: ParsedKey::Char(value.value()),
                tokens: value.to_token_stream(),
            }),
            Expr::Lit(ExprLit {
                lit: Lit::Bool(value),
                ..
            }) => Ok(Key {
                parsed: ParsedKey::Bool(value.value),
                tokens: value.to_token_stream(),
            }),
            other => Err(Error::new_spanned(
                other,
                "unsupported ReX static_map key expression",
            )),
        }
    }
}

impl PhfHash for Key {
    fn phf_hash<H: Hasher>(&self, state: &mut H) {
        self.parsed.phf_hash(state);
    }
}

struct Entry {
    key: Key,
    value: Expr,
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key = Key::from_expr(input.parse()?)?;
        input.parse::<Token![=>]>()?;
        let value = input.parse()?;
        Ok(Entry { key, value })
    }
}

impl PhfHash for Entry {
    fn phf_hash<H: Hasher>(&self, state: &mut H) {
        self.key.phf_hash(state);
    }
}

struct Entries(Vec<Entry>);

impl Parse for Entries {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            entries.push(input.parse()?);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let mut unique = HashSet::new();
        for entry in &entries {
            if !unique.insert(entry.key.parsed.clone()) {
                return Err(Error::new_spanned(
                    entry.key.tokens.clone(),
                    "duplicate static_map key",
                ));
            }
        }

        Ok(Entries(entries))
    }
}

#[proc_macro]
pub fn static_map_phf(input: TokenStream) -> TokenStream {
    let Entries(entries) = parse_macro_input!(input as Entries);
    let state = phf_generator::generate_hash(&entries);
    build_map(&entries, state).into()
}

fn build_map(entries: &[Entry], state: HashState) -> TokenStream2 {
    let hash_key = state.key;
    let disps = state.disps.iter().map(|&(d1, d2)| quote!((#d1, #d2)));
    let ordered_entries = state.map.iter().map(|&index| {
        let key = &entries[index].key.tokens;
        let value = &entries[index].value;
        quote!((#key, #value))
    });

    quote! {
        ::static_map::Map {
            key: #hash_key,
            disps: &[#(#disps),*],
            entries: &[#(#ordered_entries),*],
        }
    }
}
