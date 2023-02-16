use proc_macro::TokenStream;
use proc_macro_util::ok_or_return_compile_error;

#[proc_macro_derive(Context, attributes(context, decision_maker, storage_client, subject))]
pub fn context(item: TokenStream) -> TokenStream {
    ok_or_return_compile_error!(core::context(item.into())).into()
}
