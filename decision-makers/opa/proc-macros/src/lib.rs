#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

use authzen_proc_macro_util::ok_or_return_compile_error;
use core::*;
use proc_macro::TokenStream;

#[proc_macro_derive(OPAContext, attributes(account_session, opa_client))]
pub fn opa_context(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(opa_context_core(item.into())).into()
}

#[proc_macro_derive(OPATxCacheContext, attributes(db, opa_tx_cache_client))]
pub fn opa_tx_cache_context(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(opa_tx_cache_context_core(item.into())).into()
}

#[proc_macro_derive(OPAType)]
pub fn opa_type(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(opa_type_core(item.into())).into()
}
