#![allow(unused_imports)]
extern crate proc_macro;
use quote::{format_ident, quote, ToTokens};
use std::any::Any;
use syn::parse::{Parse, ParseStream, Result};
use syn::token::Token;
use syn::{
    fold::{fold_ident, Fold},
    visit_mut::{self, VisitMut},
    ExprLit, Lit,
};
use syn::{parse_macro_input, Expr, Ident, Local, Pat, Stmt, Token};

const CLONE_NAME: &str = "_clone__fate__";
const VALUE_NAME: &str = "_value__fate__";
#[derive(Default)]
struct CloneFold {
    pub(crate) clones: String,
    pub(crate) values: String,
    pub(crate) dependencies: String,
}
impl Fold for CloneFold {
    fn fold_ident(&mut self, i: Ident) -> Ident {
        let clone_ident = format_ident!("{}{}", i, CLONE_NAME);
        let value_ident = format_ident!("{}{}", i, VALUE_NAME);

        self.clones += &format!("let {} = {}.clone(); ", clone_ident, i);
        self.dependencies += &format!("{}.clone(), ", i);
        let value_expr_str =
            &format!("let {} = {}.get_value();", value_ident, clone_ident);
        if !self.values.contains(value_expr_str) {
            self.values += value_expr_str;
        }
        value_ident
    }
}

struct Fate {
    quotes: Vec<proc_macro2::TokenStream>,
}
impl Parse for Fate {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut quotes: Vec<proc_macro2::TokenStream> = Vec::new();
        while !input.is_empty() {
            let is_new = if input.peek(Token![let]) {
                input.parse::<Token![let]>()?;
                true
            } else {
                false
            };
            let fate_ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let expr = input.parse::<Expr>()?;
            input.parse::<Token![;]>()?;

            let mut clone_fold = CloneFold::default();
            let fixed_expr = clone_fold.fold_expr(expr);

            let clones: proc_macro2::TokenStream = clone_fold.clones.parse().unwrap();
            let dependencies: proc_macro2::TokenStream =
                clone_fold.dependencies.parse().unwrap();
            let value_expr: proc_macro2::TokenStream =
                clone_fold.values.parse().unwrap();

            let binding_quote = if is_new {
                quote! {
                    let #fate_ident = Fate::from_expression
                }
            } else {
                quote! {
                    #fate_ident.bind_expression
                }
            };

            let quote = quote! {
                #clones;
                #binding_quote(
                    Box::new(move || {#value_expr #fixed_expr}), vec![#dependencies]);
            };
            quotes.push(quote);
        }

        Ok(Fate { quotes })
    }
}

#[proc_macro]
pub fn fate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Fate { quotes } = parse_macro_input!(input as Fate);

    let expanded = quote! {
        #(#quotes)*
    };

    eprintln!("{}", expanded);

    proc_macro::TokenStream::from(expanded)
}
