#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

use doc_comment::doc_comment;
use proc_macro::TokenStream;
use proc_macro_util::ok_or_return_compile_error;

doc_comment!(
    include_str!("../docs/action.md"),
    #[proc_macro]
    pub fn action(item: TokenStream) -> TokenStream {
        ok_or_return_compile_error!(core::action(item.into())).into()
    }
);

#[proc_macro_derive(AuthzObject, attributes(authzen))]
pub fn authz_object(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(core::authz_object(item.into())).into()
}

#[proc_macro_derive(Context, attributes(context, decision_maker, storage_client, subject))]
pub fn context(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(core::context(item.into())).into()
}
