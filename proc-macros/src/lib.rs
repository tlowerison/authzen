#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

use authzen_proc_macro_util::ok_or_return_compile_error;
use doc_comment::doc_comment;
use proc_macro::TokenStream;

doc_comment!(
    include_str!("../docs/action.md"),
    #[proc_macro]
    pub fn action(item: TokenStream) -> TokenStream {
        ok_or_return_compile_error!(authzen_proc_macros_core::action(item.into())).into()
    }
);

#[proc_macro_derive(AuthzObject, attributes(authzen))]
pub fn authz_object(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(authzen_proc_macros_core::authz_object(item.into())).into()
}

#[proc_macro_derive(
    Context,
    attributes(context, decision_maker, storage_client, subject, transaction_cache)
)]
pub fn context(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(authzen_proc_macros_core::context(item.into())).into()
}
