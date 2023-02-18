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

    let can_trait_name = format_ident!("Can{name}");
    let can_fn_name = format_ident!("can_{snake_name}");

    let try_trait_name = format_ident!("Try{name}");
    let try_fn_name = format_ident!("try_{snake_name}");

    let can_trait_doc = format!(
        r#"
        Represents a query whether the action with type `"{ty}"` can be performed
        given the provided input and context (which must provide information about
        the event's subject, context and decision maker). Automatically implmented
        for any object which can be queried with the given [`DecisionMaker`].
        "#
    );
    let can_fn_doc = format!("Query whether the subject is authorized to {ty} the specified object(s).");

    let try_trait_doc = format!(
        r#"
        Makes an authorization query and performs `"{ty}"` action.

        Includes:
        - a query whether the action with type `"{ty}"` can be performed
          given the provided input and context (which must provide information about
          the event's subject, context and decision maker). Automatically implmented
          for any object which can be queried about for the given [`DecisionMaker`].
        - upon approval of the action by the specified [`DecisionMaker`], the action
          is actually performed
        "#
    );
    let try_fn_doc =
        format!("Query whether the subject is authorized to {ty} the specified object(s). If so, perform the action.");

    let tokens = quote! {
        #[doc(hidden)]
        #[derive(#source_mod derivative::Derivative)]
        #[derivative(Clone(bound = ""), Copy(bound = ""), Debug, Default(bound = ""))]
        pub struct #name<O: ?Sized>(#[derivative(Debug = "ignore")] std::marker::PhantomData<O>);

        impl<O: ?Sized> #source_mod ActionType for #name<O> {
            const TYPE: &'static str = #ty;
        }

        #[doc = #can_trait_doc]
        pub trait #can_trait_name<'subject, 'context, 'input, Ctx, I>: Send + Sync
        where
            Ctx: Sync + 'subject + 'context,
            I: Send + Sync + 'input,
        {
            #[doc = #can_fn_doc]
            fn #can_fn_name<'life0, 'async_trait, DM>(
                ctx: &'life0 Ctx,
                input: &'input I,
            ) -> std::pin::Pin<Box<dyn std::future::Future<
                Output = Result<
                    <DM as #source_mod DecisionMaker<
                        <Ctx as #source_mod AuthorizationContext<DM>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        &'input I,
                        <Ctx as #source_mod AuthorizationContext<DM>>::Context<'context>,
                    >>::Ok,
                    <DM as #source_mod DecisionMaker<
                        <Ctx as #source_mod AuthorizationContext<DM>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        &'input I,
                        <Ctx as #source_mod AuthorizationContext<DM>>::Context<'context>,
                    >>::Error,
                >,
            > + 'async_trait>>
            where
                Ctx: #source_mod AuthorizationContext<DM>,
                DM: #source_mod DecisionMaker<
                        <Ctx as #source_mod AuthorizationContext<DM>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        &'input I,
                        <Ctx as #source_mod AuthorizationContext<DM>>::Context<'context>,
                    > + Sync,

                'subject: 'async_trait,
                'context: 'async_trait,
                'input: 'async_trait,
                'life0: 'async_trait + 'subject + 'context + 'input,
                Self: 'async_trait,
                DM: 'async_trait,
                I: 'async_trait,
            {
                use #source_mod AuthorizationContext;
                let fut: std::pin::Pin<Box<dyn std::future::Future<Output = _> + 'async_trait>> = Box::pin(async move {
                    ctx.decision_maker().can_act(&#source_mod Event {
                        context: ctx.context(),
                        subject: ctx.subject(),
                        action: std::marker::PhantomData::<#name<Self>>::default(),
                        object: std::marker::PhantomData::<Self>::default(),
                        input,
                    }).await
                });
                fut
            }
        }

        #[doc = #try_trait_doc]
        pub trait #try_trait_name<'subject, 'context, Ctx, I>: Send + Sync
        where
            Ctx: Sync + 'subject + 'context,
            I: Send + Sync,
        {
            #[doc = #try_fn_doc]
            fn #try_fn_name<'life0, 'async_trait, DM, SC>(
                ctx: &'life0 Ctx,
                input: I,
            ) -> std::pin::Pin<Box<dyn std::future::Future<
                Output = Result<
                    <#name<Self> as #source_mod StorageAction<SC, I>>::Ok,
                    #source_mod ActionError<
                        <DM as #source_mod DecisionMaker<
                            <Ctx as #source_mod AuthorizationContext<DM>>::Subject<'subject>,
                            #name<Self>,
                            Self,
                            I,
                            <Ctx as #source_mod AuthorizationContext<DM>>::Context<'context>,
                        >>::Error,
                        <#name<Self> as #source_mod StorageAction<SC, I>>::Error,
                    >,
                >,
            > + 'async_trait>>
            where
                Self: #source_mod AsStorage<<SC as StorageClient>::Backend>,
                DM: #source_mod DecisionMaker<
                        <Ctx as #source_mod AuthorizationContext<DM>>::Subject<'subject>,
                        #name<Self>,
                        Self,
                        I,
                        <Ctx as #source_mod AuthorizationContext<DM>>::Context<'context>,
                    > + Sync,
                SC: #source_mod StorageClient + Send + Sync,
                Ctx: #source_mod TryActionContext<DM, SC>,
                #name<Self>: #source_mod StorageAction<SC, I>,

                'subject: 'async_trait,
                'context: 'async_trait,
                'life0: 'async_trait + 'subject + 'context,
                Self: 'async_trait,
                SC: 'async_trait,
                DM: 'async_trait,
                I: 'async_trait,
            {
                use #source_mod TryAct;
                use #source_mod TryActionContext;
                let event = #source_mod Event {
                    context: ctx.context(),
                    subject: ctx.subject(),
                    action: std::marker::PhantomData::<#name<Self>>::default(),
                    object: std::marker::PhantomData::<Self>::default(),
                    input,
                };
                Box::pin(event.try_act(ctx.decision_maker(), ctx.storage_client()))
            }
        }

        impl<'subject, 'context, 'input, Ctx, T, I> #can_trait_name<'subject, 'context, 'input, Ctx, I> for T
        where
            Self: Send + Sync,
            Ctx: Sync + 'subject + 'context,
            I: Send + Sync + 'input,
        {
        }

        impl<'subject, 'context, Ctx, T, I> #try_trait_name<'subject, 'context, Ctx, I> for T
        where
            Self: Send + Sync,
            Ctx: Sync + 'subject + 'context,
            I: Send + Sync,
        {
        }
    };

    Ok(tokens)
}
