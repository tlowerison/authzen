#![feature(proc_macro_span)]

use proc_macro::TokenStream;

#[proc_macro_derive(Audit, attributes(audit))]
pub fn derive_audit(tokens: TokenStream) -> TokenStream {
    match core::derive_audit(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(Db, attributes(db))]
pub fn derive_db(tokens: TokenStream) -> TokenStream {
    match core::derive_db(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(Enum, attributes(id))]
pub fn derive_enum(tokens: TokenStream) -> TokenStream {
    match core::derive_enum(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(IncludesChanges)]
pub fn derive_includes_changes(tokens: TokenStream) -> TokenStream {
    match core::derive_includes_changes(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(SoftDelete)]
pub fn derive_soft_delete(tokens: TokenStream) -> TokenStream {
    match core::derive_soft_delete(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro]
pub fn db_filter(tokens: TokenStream) -> TokenStream {
    match core::db_filter(tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro]
pub fn dynamic_schema(tokens: TokenStream) -> TokenStream {
    use std::ops::Deref;
    use std::str::FromStr;

    let core::DynamicSchema {
        ident,
        schema_relative_file_path,
    } = syn::parse_macro_input!(tokens as core::DynamicSchema);
    let schema_relative_path_str = schema_relative_file_path.value();
    let schema_relative_path = std::path::Path::new(&schema_relative_path_str);
    let source_path = proc_macro::Span::call_site().source_file().path();

    let source_dir = match source_path.parent() {
        Some(source_dir) => source_dir,
        None => {
            return syn::parse::Error::new(
                proc_macro2::Span::call_site(),
                "cannot determine call site's parent directory",
            )
            .into_compile_error()
            .into()
        }
    };
    let schema_path = source_dir.join(schema_relative_path);

    let bytes = match std::fs::read(&schema_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return syn::Error::new(
                schema_relative_file_path.span(),
                format!("could not read file located at `{}`", schema_path.display()),
            )
            .into_compile_error()
            .into()
        }
    };

    let tokens = match TokenStream::from_str(String::from_utf8_lossy(&bytes).deref()) {
        Ok(tokens) => tokens,
        Err(_) => {
            return syn::parse::Error::new(
                proc_macro2::Span::call_site(),
                format!("unable to parse file located at `{}`", schema_path.display()),
            )
            .into_compile_error()
            .into()
        }
    };

    match core::dynamic_schema(ident, tokens.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// container level attribute used by [`SoftDelete`]
#[proc_macro_attribute]
pub fn soft_delete(_: TokenStream, item: TokenStream) -> TokenStream {
    item
}
