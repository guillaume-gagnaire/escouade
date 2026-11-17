//! The single list of the app's commands (docs/REMOTE.md, « La fondation commune », point 2): each command is written
//! once, and served by Tauri's IPC and by the remote API alike, so that the two never part.
//!
//! - [`macro@command`] stands for `#[tauri::command]`: it gives the function to Tauri's own attribute as it is, and
//!   writes beside it `__remote_<name>`, which calls it for the remote API (`crate::remoteapi`): its arguments read from
//!   a JSON object by the same names as the IPC's (camelCase), the states and the channels from the remote call, its
//!   result turned into the IPC's answer. `commands.rs` names it `tauri::command` (its module `tauri`), so that a
//!   command is written there as it always was.
//! - [`commands!`] takes the list of the commands, sorted by what the remote API does of each, and makes both the
//!   IPC's handler (`tauri::generate_handler!`) and the remote API's table.

use heck::ToLowerCamelCase;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, FnArg, Ident, ItemFn, Pat, Path, Token};

/// `#[tauri::command]`, and the call of the function by the remote API.
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = Tokens::from(attr);
    let func = syn::parse_macro_input!(item as ItemFn);
    // Tauri's names of the arguments: camelCase, unless `rename_all = "snake_case"`.
    let snake = attr
        .to_string()
        .replace(' ', "")
        .contains("rename_all=\"snake_case\"");
    let adapter = adapter(&func, snake);
    // Tauri's attribute in the desktop app alone: without its feature `desktop`, the app's crate has no IPC (escouade-server,
    // docs/CRATES.md, § 14), the remote API's call is the command's only one.
    let tauri = if attr.is_empty() {
        quote!(#[cfg_attr(feature = "desktop", ::tauri::command)])
    } else {
        quote!(#[cfg_attr(feature = "desktop", ::tauri::command(#attr))])
    };
    quote! {
        #tauri
        #func
        #adapter
    }
    .into()
}

/// `__remote_<name>`: the function called for the remote API, its arguments taken from the call.
fn adapter(func: &ItemFn, snake: bool) -> Tokens {
    let name = &func.sig.ident;
    let remote = format_ident!("__remote_{}", name);
    let vis = &func.vis;
    let refused = |why: &str| {
        quote! {
            #[doc(hidden)]
            #vis fn #remote(_call: crate::remoteapi::Call) -> crate::remoteapi::Reply {
                crate::remoteapi::Reply::refused(#why)
            }
        }
    };
    // A command generic over Tauri's runtime is the desktop's: the remote API has no runtime to give it.
    if !func.sig.generics.params.is_empty() {
        return refused("generic");
    }
    let mut lets = Vec::new();
    let mut args = Vec::new();
    for (i, input) in func.sig.inputs.iter().enumerate() {
        let FnArg::Typed(typed) = input else {
            return refused("method");
        };
        let ident = match &*typed.pat {
            Pat::Ident(p) => p.ident.clone(),
            _ => return refused("pattern"),
        };
        let key = if snake {
            ident.to_string().trim_start_matches('_').to_string()
        } else {
            ident.to_string().to_lower_camel_case()
        };
        let ty = &typed.ty;
        let var = Ident::new(&format!("__arg{i}"), Span::call_site());
        lets.push(quote! {
            let #var = {
                #[allow(unused_imports)]
                use crate::remoteapi::arg::{ViaCall as _, ViaJson as _};
                (&crate::remoteapi::arg::Tag::<#ty>::new()).take(__call, #key)?
            };
        });
        args.push(var);
    }
    let call = if func.sig.asyncness.is_some() {
        quote!(#name(#(#args),*).await)
    } else {
        quote!(#name(#(#args),*))
    };
    let body = quote! {
        let __call = &__call;
        #(#lets)*
        let __out = #call;
        #[allow(unused_imports)]
        use crate::remoteapi::out::{ViaResult as _, ViaValue as _};
        (&crate::remoteapi::out::Out::new(__out)).reply()
    };
    let reply = if func.sig.asyncness.is_some() {
        quote!(crate::remoteapi::Reply::future(async move { #body }))
    } else {
        quote!(crate::remoteapi::Reply::blocking(move || { #body }))
    };
    quote! {
        #[doc(hidden)]
        #[allow(clippy::all, unused_qualifications)]
        #vis fn #remote(__call: crate::remoteapi::Call) -> crate::remoteapi::Reply {
            #reply
        }
    }
}

/// One group of the list: its class and its commands.
struct Group {
    class: Ident,
    paths: Punctuated<Path, Token![,]>,
}

struct List(Vec<Group>);

impl Parse for List {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut groups = Vec::new();
        while !input.is_empty() {
            let class: Ident = input.parse()?;
            let content;
            braced!(content in input);
            let paths = Punctuated::parse_terminated(&content)?;
            groups.push(Group { class, paths });
        }
        Ok(List(groups))
    }
}

/// The list of the commands, by class: `served { commands::a, … } desk { … } moves_served { … } moves_refused { … }`.
/// It makes, in the module it is written in:
/// - `ipc()`, the IPC's handler of all of them (`tauri::generate_handler!`);
/// - `IPC`, the names it was given, in the list's order;
/// - `COMMANDS`, the remote API's table: each one's name, class and `__remote_<name>`.
#[proc_macro]
pub fn commands(input: TokenStream) -> TokenStream {
    let List(groups) = syn::parse_macro_input!(input as List);
    let mut all = Vec::new();
    let mut names = Vec::new();
    let mut rows = Vec::new();
    // The same, in the core without a window (no feature `desktop`): the commands refused at a distance (the desktop's,
    // and those that change machines and are not served yet) are refused there by their class, never called: their
    // functions, which may need the window, are not built (`remoteapi::refused_here`).
    let mut rows_alone = Vec::new();
    let mut seen = BTreeSet::new();
    for group in &groups {
        let class = match group.class.to_string().as_str() {
            "served" => quote!(crate::remoteapi::Class::Served),
            "desk" => quote!(crate::remoteapi::Class::Desk),
            "moves_served" => quote!(crate::remoteapi::Class::Moves { served: true }),
            "moves_refused" => quote!(crate::remoteapi::Class::Moves { served: false }),
            other => {
                return syn::Error::new_spanned(
                    &group.class,
                    format!("unknown class `{other}`: served, desk, moves_served or moves_refused"),
                )
                .to_compile_error()
                .into()
            }
        };
        for path in &group.paths {
            let last = path.segments.last().expect("a path has a segment");
            let name = last.ident.to_string();
            if !seen.insert(name.clone()) {
                return syn::Error::new_spanned(path, format!("`{name}` is twice in the list"))
                    .to_compile_error()
                    .into();
            }
            let mut remote = path.clone();
            remote.segments.last_mut().unwrap().ident = format_ident!("__remote_{}", last.ident);
            all.push(path.clone());
            names.push(name.clone());
            rows.push(quote! {
                crate::remoteapi::Command { name: #name, class: #class, call: #remote }
            });
            let refused = matches!(group.class.to_string().as_str(), "desk" | "moves_refused");
            rows_alone.push(if refused {
                quote! {
                    crate::remoteapi::Command { name: #name, class: #class, call: crate::remoteapi::refused_here }
                }
            } else {
                quote! {
                    crate::remoteapi::Command { name: #name, class: #class, call: #remote }
                }
            });
        }
    }
    quote! {
        /// The IPC's handler of every command of the list.
        #[cfg(feature = "desktop")]
        pub fn ipc() -> impl Fn(::tauri::ipc::Invoke<::tauri::Wry>) -> bool + Send + Sync + 'static {
            ::tauri::generate_handler![#(#all),*]
        }

        /// The names the IPC's handler was given, in the list's order.
        pub const IPC: &[&str] = &[#(#names),*];

        /// The remote API's table: each command of the list, its class and its call.
        #[cfg(feature = "desktop")]
        pub static COMMANDS: &[crate::remoteapi::Command] = &[#(#rows),*];

        /// The remote API's table in the core without a window: the commands it refuses are not built there.
        #[cfg(not(feature = "desktop"))]
        pub static COMMANDS: &[crate::remoteapi::Command] = &[#(#rows_alone),*];
    }
    .into()
}
