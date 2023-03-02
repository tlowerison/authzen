use crate::*;
use authzen::actions::*;
use authzen::*;
use service_util::Error;

#[instrument]
pub async fn create_item<D: Db>(
    ctx: &CtxOptSession<'_, D>,
    name: String,
    description: Option<String>,
) -> Result<DbItem, Error> {
    Ok(Item::try_create_one(ctx, DbItem::builder().name(name).description(description).build()).await?)
}
