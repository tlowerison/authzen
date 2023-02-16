diesel::table! {
    account (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamp>,
        username -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
    }
}

diesel::table! {
    account_audit (id) {
        id -> Uuid,
        account_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamp>,
        username -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
    }
}

diesel::table! {
    app.item (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> Varchar,
        description -> Nullable<Varchar>,
    }
}

diesel::table! {
    app.item_audit (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> Varchar,
        description -> Nullable<Varchar>,
        item_id_arbitrary_foreign_key_name -> Uuid,
    }
}

diesel::table! {
    app.cart (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        account_id -> Uuid,
    }
}

diesel::table! {
    app.cart_item (id) {
        id -> Uuid,
        created_at -> Timestamp,
        cart_id -> Uuid,
        item_id -> Uuid,
    }
}

diesel::joinable!(account_audit -> account (account_id));
diesel::joinable!(cart -> account (account_id));
diesel::joinable!(cart_item -> cart (cart_id));
diesel::joinable!(item_audit -> item (item_id_arbitrary_foreign_key_name));

diesel::allow_tables_to_appear_in_same_query!(account, cart, cart_item, item, item_audit,);
