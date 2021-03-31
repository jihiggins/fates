#![allow(unused_imports)]
extern crate proc_macro;
use quote::{quote, ToTokens};
use std::any::Any;
use syn::parse::{Parse, ParseStream, Result};
use syn::token::Token;
use syn::{parse_macro_input, Expr, Ident, Local, Pat, Stmt, Token};

struct Fate {
    variables: Vec<Ident>,
    expressions: Vec<proc_macro2::TokenStream>,
    dependencies: Vec<proc_macro2::TokenStream>,
    clone_lists: Vec<proc_macro2::TokenStream>,
}
const CLONE_NAME: &str = "_clone__fate__";
impl Parse for Fate {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut variables = Vec::<Ident>::new();
        let mut expressions = Vec::<proc_macro2::TokenStream>::new();
        let mut dependencies = Vec::<proc_macro2::TokenStream>::new();
        let mut clone_lists = Vec::<proc_macro2::TokenStream>::new();
        while !input.is_empty() {
            let is_new = if input.peek(Token![let]) {
                input.parse::<Token![let]>()?;
                true
            } else {
                false
            };
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let expr = input.parse::<Expr>()?;
            input.parse::<Token![;]>()?;

            let mut expr_str = expr.to_token_stream().to_string();
            let mut dependency_set = Vec::<Ident>::new();
            for var in &variables {
                if expr_str.find(&var.to_string()).is_some() {
                    dependency_set.push(var.clone());
                }

                expr_str = expr_str.replace(
                    &var.to_string(),
                    &format!("{}{}.get_value()", var.to_string(), CLONE_NAME),
                );
            }

            let mut dependency_string = String::from("vec![");
            let mut clones = String::new();
            for dep in dependency_set {
                let dep_string = dep.to_string();
                dependency_string += &dep_string;
                dependency_string += ".clone(), ";

                //let #cloned_variables = #variables.clone();
                clones += &format!(
                    "let {}{} = {}.clone();",
                    &dep_string, CLONE_NAME, &dep_string
                );
            }
            dependency_string += "]";
            let expr: proc_macro2::TokenStream = expr_str.parse().unwrap();
            let dep: proc_macro2::TokenStream = dependency_string.parse().unwrap();
            let clones: proc_macro2::TokenStream = clones.parse().unwrap();

            variables.push(ident);
            expressions.push(expr);
            dependencies.push(dep);
            clone_lists.push(clones);
        }

        Ok(Fate {
            variables,
            expressions,
            dependencies,
            clone_lists,
        })
    }
}

#[proc_macro]
pub fn fate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Fate {
        variables,
        expressions,
        dependencies,
        clone_lists,
    } = parse_macro_input!(input as Fate);

    let expanded = quote! {
        #(
            #clone_lists;
            let #variables = Fate::from_expression(
                Box::new(move || #expressions), #dependencies);
        )*
    };

    // eprintln!("{}", expanded);

    proc_macro::TokenStream::from(expanded)
}
