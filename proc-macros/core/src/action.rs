use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{parse2, Error, Token};

#[derive(Clone, Debug)]
pub struct ActionArgs {
    pub name: syn::Ident,
    pub ty: Option<String>,
    pub internal: bool,
}

impl Parse for ActionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut internal = false;
        let mut name = input.parse::<syn::Ident>()?;

        if name == "__authzen_internal" {
            internal = true;
            input.parse::<Token![,]>()?;
            name = input.parse::<syn::Ident>()?;
        }

        if input.is_empty() {
            return Ok(Self {
                name,
                internal,
                ty: None,
            });
        }
        input.parse::<Token![=]>()?;
        let ty = input.parse::<syn::LitStr>()?;
        Ok(Self {
            name,
            internal,
            ty: Some(ty.value()),
        })
    }
}

pub fn action(item: TokenStream) -> Result<TokenStream, Error> {
    let ActionArgs { name, ty, internal } = parse2(item)?;

    let snake_name = name.to_string().to_case(Case::Snake);
    let ty = ty.unwrap_or_else(|| snake_name.clone());

    let source_mod = if internal { quote!() } else { quote!(authzen::) };
    let data_sources_source_mod = if internal {
        quote!(authzen_data_sources::)
    } else {
        quote!(authzen::data_sources::)
    };

    let can_fn_name = format_ident!("can_{snake_name}");

    let try_trait_name = format_ident!("Try{name}");
    let try_fn_name = format_ident!("try_{snake_name}");
    let try_one_fn_name = format_ident!("try_{snake_name}_one");

    let can_fn_doc = format!("Query whether the subject is authorized to {ty} the specified object(s).");

    let try_trait_doc = format!(
        r#"
        Makes an authorization query and performs `"{ty}"` action.

        Includes:
        - a query whether the action with type `"{ty}"` can be performed
          given the provided input and context (which must provide information about
          the event's subject, context and authorization engine). Automatically implmented
          for any object which can be queried about for the given [`AuthzEngine`].
        - upon approval of the action by the specified [`AuthzEngine`], the action
          is actually performed
        "#
    );
    let try_fn_doc =
        format!("Query whether the subject is authorized to {ty} the specified objects. If so, perform the action.");
    let try_one_fn_doc =
        format!("Query whether the subject is authorized to {ty} the specified object. If so, perform the action. Expects the return type of the storage action to implement [`IntoIterator`].");

    let tokens = quote! {
        #[doc(hidden)]
        #[derive(#source_mod derivative::Derivative)]
        #[derivative(Clone(bound = ""), Copy(bound = ""), Debug, Default(bound = ""))]
        pub struct #name<O: ?Sized>(#[derivative(Debug = "ignore")] std::marker::PhantomData<O>);

        impl<O: ?Sized> #source_mod ActionType for #name<O> {
            const TYPE: &'static str = #ty;
        }

        #[doc = #try_trait_doc]
        pub trait #try_trait_name<'subject, 'context, 'input, Ctx>: Send + Sync
        where
            Ctx: Sync + 'subject + 'context,
        {
            #[doc = #can_fn_doc]
            fn #can_fn_name<'life0, 'async_trait, AE, DS, TC, I>(
                ctx: &'life0 Ctx,
                input: &'input I,
            ) -> std::pin::Pin<Box<dyn std::future::Future<
                Output = Result<
                    <AE as #source_mod AuthzEngine<
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        I,
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                        DS::TransactionId,
                    >>::Ok,
                    <AE as #source_mod AuthzEngine<
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        I,
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                        DS::TransactionId,
                    >>::Error,
                >,
            > + Send + 'async_trait>>
            where
                Self: #source_mod AsStorage<<DS as #data_sources_source_mod DataSource>::Backend>,
                AE: #source_mod AuthzEngine<
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        I,
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                        DS::TransactionId,
                    > + Sync,
                DS: #data_sources_source_mod DataSource + Send + Sync,
                TC: Send + Sync + #source_mod TransactionCache,
                Ctx: #source_mod AuthorizationContext<AE, DS, TC>,
                I: Send + Sync,

                'subject: 'async_trait,
                'context: 'async_trait + 'input,
                'input: 'async_trait,
                'life0: 'async_trait + 'subject + 'context,
                Self: 'async_trait,
                AE: 'async_trait,
                DS: 'async_trait,
                TC: 'async_trait,
                I: 'async_trait,
            {
                use #source_mod AuthorizationContext;
                Box::pin(<AE as AuthzEngine<
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        I,
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                        DS::TransactionId,
                    >>::can_act(ctx.authz_engine(), ctx.subject(), input, ctx.context(), ctx.data_source().transaction_id()))
            }

            #[doc = #try_fn_doc]
            fn #try_fn_name<'life0, 'async_trait, AE, DS, TC, I>(
                ctx: &'life0 Ctx,
                input: I,
            ) -> std::pin::Pin<Box<dyn std::future::Future<
                Output = Result<
                    <#name<Self> as #source_mod StorageAction<DS, I>>::Ok,
                    #source_mod ActionError<
                        <AE as #source_mod AuthzEngine<
                            <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                            #name<Self>,
                            Self,
                            I,
                            <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                            DS::TransactionId,
                        >>::Error,
                        <#name<Self> as #source_mod StorageAction<DS, I>>::Error,
                        <TC as #source_mod TransactionCache>::Error,
                    >,
                >,
            > + Send + 'async_trait>>
            where
                Self: #source_mod AsStorage<<DS as #data_sources_source_mod DataSource>::Backend>,
                AE: #source_mod AuthzEngine<
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        I,
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                        DS::TransactionId,
                    > + Sync,
                DS: #data_sources_source_mod DataSource + Send + Sync,
                TC: Send + Sync
                    + #source_mod TransactionCache
                    + #source_mod TransactionCacheAction<#name<Self>, DS, I>,
                Ctx: #source_mod AuthorizationContext<AE, DS, TC>,
                #name<Self>: #source_mod StorageAction<DS, I>,
                I: Send + Sync,

                'subject: 'async_trait,
                'context: 'async_trait,
                'input: 'async_trait,
                'life0: 'async_trait + 'subject + 'context,
                Self: 'async_trait,
                AE: 'async_trait,
                DS: 'async_trait,
                TC: 'async_trait,
                I: 'async_trait,
            {
                use #source_mod AuthorizationContext;
                use #source_mod TransactionCache;
                use #source_mod TryAct;
                let event = #source_mod Event {
                    context: ctx.context(),
                    subject: ctx.subject(),
                    action: std::marker::PhantomData::<#name<Self>>::default(),
                    object: std::marker::PhantomData::<Self>::default(),
                    input,
                };
                Box::pin(event.try_act(ctx.authz_engine(), ctx.data_source(), ctx.transaction_cache()))
            }

            #[doc = #try_one_fn_doc]
            fn #try_one_fn_name<'life0, 'async_trait, AE, DS, TC, I>(
                ctx: &'life0 Ctx,
                input: I,
            ) -> std::pin::Pin<Box<dyn std::future::Future<
                Output = Result<
                    <<#name<Self> as #source_mod StorageAction<DS, [I; 1]>>::Ok as IntoIterator>::Item,
                    #source_mod ActionError<
                        <AE as #source_mod AuthzEngine<
                            <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                            #name<Self>,
                            Self,
                            [I; 1],
                            <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                            DS::TransactionId,
                        >>::Error,
                        <#name<Self> as #source_mod StorageAction<DS, [I; 1]>>::Error,
                        <TC as #source_mod TransactionCache>::Error,
                    >,
                >,
            > + Send + 'async_trait>>
            where
                Self: #source_mod AsStorage<<DS as #data_sources_source_mod DataSource>::Backend>,
                AE: #source_mod AuthzEngine<
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        [I; 1],
                        <Ctx as #source_mod AuthorizationContext<AE, DS, TC>>::Context<'context>,
                        DS::TransactionId,
                    > + Sync,
                DS: #data_sources_source_mod DataSource + Send + Sync,
                TC: Send + Sync
                    + #source_mod TransactionCache
                    + #source_mod TransactionCacheAction<#name<Self>, DS, [I; 1]>,
                Ctx: #source_mod AuthorizationContext<AE, DS, TC>,
                #name<Self>: #source_mod StorageAction<DS, [I; 1]>,
                I: Send + Sync,

                <#name<Self> as #source_mod StorageAction<DS, [I; 1]>>::Ok: IntoIterator,
                <<#name<Self> as #source_mod StorageAction<DS, [I; 1]>>::Ok as IntoIterator>::Item: Send,

                'subject: 'async_trait,
                'context: 'async_trait,
                'input: 'async_trait,
                'life0: 'async_trait + 'subject + 'context,
                Self: 'async_trait,
                AE: 'async_trait,
                DS: 'async_trait,
                TC: 'async_trait,
                I: 'async_trait,
            {
                use #source_mod futures::future::{ready, TryFutureExt};
                use #source_mod AuthorizationContext;
                use #source_mod TransactionCache;
                use #source_mod TryAct;
                let event = #source_mod Event {
                    context: ctx.context(),
                    subject: ctx.subject(),
                    action: std::marker::PhantomData::<#name<Self>>::default(),
                    object: std::marker::PhantomData::<Self>::default(),
                    input: [input],
                };
                Box::pin(
                    event.try_act(ctx.authz_engine(), ctx.data_source(), ctx.transaction_cache())
                        .and_then(|ok| {
                            let mut iter = ok.into_iter();
                            ready(iter.next().ok_or_else(|| #source_mod ActionError::DataSource(<#name<Self> as #source_mod StorageAction<DS, [I; 1]>>::Error::not_found())))
                        })
                )
            }
        }

        impl<'subject, 'context, 'input, Ctx, T> #try_trait_name<'subject, 'context, 'input, Ctx> for T
        where
            Self: Send + Sync,
            Ctx: Sync + 'subject + 'context,
        {
        }
    };

    Ok(tokens)
}
