use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Expr, ExprLit, Lit, LitInt, Result, Token};

struct Entry {
    key: Expr,
    value: Expr,
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=>]>()?;
        let value = input.parse()?;
        Ok(Entry { key, value })
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
        Ok(Entries(entries))
    }
}

#[proc_macro]
pub fn static_map_phf(input: TokenStream) -> TokenStream {
    let Entries(entries) = parse_macro_input!(input as Entries);
    let keys = entries.iter().map(|entry| normalize_key(&entry.key));
    let values = entries.iter().map(|entry| &entry.value);

    quote! {
        ::phf::phf_map! {
            #(#keys => #values),*
        }
    }
    .into()
}

fn normalize_key(key: &Expr) -> proc_macro2::TokenStream {
    if let Expr::Lit(ExprLit {
        lit: Lit::Int(integer),
        ..
    }) = key
    {
        if integer.suffix().is_empty() {
            let value = integer
                .base10_parse::<u32>()
                .expect("ReX static_map compatibility only supports u32 integer keys");
            let typed = LitInt::new(&format!("{}u32", value), integer.span());
            return quote!(#typed);
        }
    }

    key.to_token_stream()
}
