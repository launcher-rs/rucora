//! # rucora-macros
//!
//! Procedural macros for the [rucora](https://crates.io/crates/rucora) framework.
//!
//! ## Macros
//!
//! | Macro | Description |
//! |-------|-------------|
//! | [`#[rucora_tool]`](attr.rucora_tool.html) | Auto-generate `Tool` impl from async fn with typed params |
//! | [`#[rucora_guard]`](attr.rucora_guard.html) | Auto-generate `InjectionGuard` impl from async fn |
//!
//! ## Quick Example
//!
//! ```rust,ignore
//! use rucora::rucora_tool;
//! use serde_json::{Value, json};
//! use rucora_core::error::ToolError;
//!
//! #[rucora_tool(name = "add", description = "Add two numbers")]
//! async fn add(a: f64, b: f64) -> Result<Value, ToolError> {
//!     Ok(json!({ "result": a + b }))
//! }
//!
//! // Generates: AddParams struct + AddTool unit struct + impl Tool
//! ```

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, LitStr, Pat, ReturnType, parse_macro_input};

/// Resolve the correct crate path for rucora types.
///
/// Tries `rucora` first, then falls back to `rucora_core` for direct core usage.
fn rucora_crate_path() -> syn::Path {
    // Try rucora first
    if let Ok(found) = crate_name("rucora") {
        match found {
            FoundCrate::Itself => return syn::parse_quote!(::rucora),
            FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, Span::call_site());
                return syn::parse_quote!(::#ident);
            }
        }
    }
    // Fall back to rucora_core
    if let Ok(found) = crate_name("rucora_core") {
        match found {
            FoundCrate::Itself => return syn::parse_quote!(::rucora_core),
            FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, Span::call_site());
                return syn::parse_quote!(::#ident);
            }
        }
    }
    // Last resort: try ::rucora (most common case when used as dependency)
    syn::parse_quote!(::rucora)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// #[rucora_tool] — Generate Tool impl from async fn
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct ToolAttrs {
    name: String,
    description: String,
}

impl syn::parse::Parse for ToolAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name: Option<String> = None;
        let mut description: Option<String> = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;

            if ident == "name" {
                let _eq: syn::Token![=] = input.parse()?;
                let value: LitStr = input.parse()?;
                name = Some(value.value());
            } else if ident == "description" {
                let _eq: syn::Token![=] = input.parse()?;
                let value: LitStr = input.parse()?;
                description = Some(value.value());
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "unknown attribute, expected `name` or `description`",
                ));
            }

            if !input.is_empty() {
                let _comma: syn::Token![,] = input.parse()?;
            }
        }

        let name = name.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "#[rucora_tool] requires `name = \"...\"`")
        })?;
        let description = description.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "#[rucora_tool] requires `description = \"...\"`",
            )
        })?;

        Ok(ToolAttrs {
            name,
            description,
        })
    }
}

/// Generate a `Tool` implementation from an async function.
///
/// Auto-creates:
/// - `{FnName}Params` struct with `Deserialize` + `JsonSchema` derives
/// - `{FnName}Tool` unit struct
/// - `impl Tool for {FnName}Tool`
///
/// # Attributes
///
/// - `name` (required): Tool name exposed to the LLM
/// - `description` (required): Human-readable description
///
/// # Example
///
/// ```rust,ignore
/// use rucora::rucora_tool;
/// use serde_json::{Value, json};
/// use rucora_core::error::ToolError;
///
/// #[rucora_tool(name = "add", description = "Add two numbers")]
/// async fn add(
///     /// First number
///     a: f64,
///     /// Second number
///     b: f64,
/// ) -> Result<Value, ToolError> {
///     Ok(json!({ "result": a + b }))
/// }
///
/// // Usage: agent.tool(AddTool);
/// ```
///
/// # Function Requirements
///
/// - Must be `async fn`
/// - Must return `Result<Value, ToolError>`
/// - Parameters must implement `Deserialize` + `JsonSchema`
/// - Doc comments on params become schema descriptions
#[proc_macro_attribute]
pub fn rucora_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as ToolAttrs);
    let input_fn = parse_macro_input!(item as ItemFn);

    match tool_impl(attrs, input_fn) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn tool_impl(attrs: ToolAttrs, func: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let rucora = rucora_crate_path();
    let tool_name = &attrs.name;
    let tool_desc = &attrs.description;
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let struct_name = format_ident!("{}Tool", to_pascal_case(&fn_name_str));
    let params_name = format_ident!("{}Params", to_pascal_case(&fn_name_str));

    // Validate return type
    match &func.sig.output {
        ReturnType::Default => {
            return Err(syn::Error::new_spanned(
                &func.sig,
                "#[rucora_tool] function must return Result<Value, ToolError>",
            ));
        }
        ReturnType::Type(_, ty) => {
            let ret_str = quote! { #ty }.to_string();
            if !ret_str.contains("Result") {
                return Err(syn::Error::new_spanned(
                    ty,
                    "#[rucora_tool] function must return Result<Value, ToolError>",
                ));
            }
        }
    }

    let (param_fields, param_names) = extract_fn_params(&func)?;
    let body = &func.block;
    let vis = &func.vis;

    let expanded = quote! {
        #[doc = concat!("Parameters for the `", #tool_name, "` tool.")]
        #[derive(::serde::Deserialize, ::schemars::JsonSchema)]
        #vis struct #params_name {
            #(#param_fields),*
        }

        #[doc = concat!("Tool: ", #tool_desc)]
        #vis struct #struct_name;

        #[::async_trait::async_trait]
        impl #rucora::core::tool::Tool for #struct_name {
            fn name(&self) -> &str {
                #tool_name
            }

            fn description(&self) -> Option<&str> {
                Some(#tool_desc)
            }

            fn categories(&self) -> &'static [#rucora::core::tool::ToolCategory] {
                &[#rucora::core::tool::ToolCategory::Basic]
            }

            fn input_schema(&self) -> ::serde_json::Value {
                let schema = ::schemars::schema_for!(#params_name);
                ::serde_json::to_value(&schema).unwrap_or_default()
            }

            async fn call(
                &self,
                input: ::serde_json::Value,
                _context: &#rucora::core::tool::types::ToolContext,
            ) -> Result<::serde_json::Value, #rucora::core::error::ToolError> {
                let params: #params_name = ::serde_json::from_value(input)
                    .map_err(|e| #rucora::core::error::ToolError::Message(format!(
                        "Invalid parameters for tool '{}': {}", #tool_name, e
                    )))?;
                let #params_name { #(#param_names),* } = params;
                #body
            }
        }
    };

    Ok(expanded)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// #[rucora_guard] — Generate InjectionGuard impl from async fn
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct GuardAttrs {
    name: String,
}

impl syn::parse::Parse for GuardAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name: Option<String> = None;
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let _eq: syn::Token![=] = input.parse()?;
            let value: LitStr = input.parse()?;
            if ident == "name" {
                name = Some(value.value());
            } else {
                return Err(syn::Error::new_spanned(ident, "expected `name`"));
            }
            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        let name = name.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "#[rucora_guard] requires `name = \"...\"`")
        })?;
        Ok(GuardAttrs { name })
    }
}

/// Generate an `InjectionGuard` implementation from an async function.
///
/// # Example
///
/// ```rust,ignore
/// use rucora::rucora_guard;
/// use rucora_core::{InjectionGuard, ScanResult, Threat};
///
/// #[rucora_guard(name = "length-limit")]
/// async fn check_length(content: &str) -> ScanResult {
///     if content.len() > 50000 {
///         ScanResult::Blocked {
///             threat: Threat::new("content_too_long", "Content exceeds length limit"),
///             confidence: 1.0,
///         }
///     } else {
///         ScanResult::Clean
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn rucora_guard(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as GuardAttrs);
    let input_fn = parse_macro_input!(item as ItemFn);
    match guard_impl(attrs, input_fn) {
        ts => ts.into(),
    }
}

fn guard_impl(attrs: GuardAttrs, func: ItemFn) -> proc_macro2::TokenStream {
    let rucora = rucora_crate_path();
    let guard_name = &attrs.name;
    let struct_name = format_ident!("{}Guard", to_pascal_case(&guard_name.replace('-', "_")));
    let fn_name = &func.sig.ident;
    let _body = &func.block;

    quote! {
        pub struct #struct_name;

        #[::async_trait::async_trait]
        impl #rucora::core::InjectionGuard for #struct_name {
            fn name(&self) -> &str {
                #guard_name
            }

            async fn scan(
                &self,
                content: &str,
            ) -> Result<#rucora::core::ScanResult, #rucora::core::AgentError> {
                #fn_name(content).await
            }
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Shared helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn extract_fn_params(
    func: &ItemFn,
) -> syn::Result<(
    Vec<proc_macro2::TokenStream>,
    Vec<syn::Ident>,
)> {
    let mut param_fields = Vec::new();
    let mut param_names = Vec::new();

    for arg in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            let pat = &pat_type.pat;
            let ty = &pat_type.ty;

            let field_name = if let Pat::Ident(pi) = pat.as_ref() {
                pi.ident.clone()
            } else {
                return Err(syn::Error::new_spanned(
                    pat,
                    "expected simple identifier parameter",
                ));
            };

            let doc_str = extract_doc_comments(&pat_type.attrs);
            let schemars_attr = if let Some(doc) = &doc_str {
                quote! { #[schemars(description = #doc)] }
            } else {
                quote! {}
            };

            param_fields.push(quote! {
                #schemars_attr
                pub #field_name: #ty
            });
            param_names.push(field_name);
        }
    }

    Ok((param_fields, param_names))
}

fn extract_doc_comments(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let syn::Meta::NameValue(nv) = &attr.meta
                && let syn::Expr::Lit(expr_lit) = &nv.value
                && let syn::Lit::Str(s) = &expr_lit.lit
            {
                return Some(s.value().trim().to_string());
            }
            None
        })
        .collect();

    if docs.is_empty() {
        None
    } else {
        Some(docs.join(" "))
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + &chars.collect::<String>()
                }
            }
        })
        .collect()
}
