(function() {var implementors = {
"authzen_core":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_core/transaction_caches/mongodb/struct.TxEntityFull.html\" title=\"struct authzen_core::transaction_caches::mongodb::TxEntityFull\">TxEntityFull</a>",1,["authzen_core::transaction_caches::mongodb::TxEntityFull"]],["impl&lt;Subject, Action: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>, Object: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>, Input, Context&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_core/struct.Event.html\" title=\"struct authzen_core::Event\">Event</a>&lt;Subject, Action, Object, Input, Context&gt;<span class=\"where fmt-newline\">where\n    Action: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    Context: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    Input: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    Object: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    Subject: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_core::Event"]],["impl&lt;T, Id&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_core/struct.TxCacheEntity.html\" title=\"struct authzen_core::TxCacheEntity\">TxCacheEntity</a>&lt;T, Id&gt;<span class=\"where fmt-newline\">where\n    Id: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_core::TxCacheEntity"]],["impl&lt;E1, E2, E3&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_core/enum.ActionError.html\" title=\"enum authzen_core::ActionError\">ActionError</a>&lt;E1, E2, E3&gt;<span class=\"where fmt-newline\">where\n    E1: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    E2: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    E3: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_core::ActionError"]]],
"authzen_diesel_core":[["impl&lt;AC, C&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/connection/struct.DbConnection.html\" title=\"struct authzen_diesel_core::connection::DbConnection\">DbConnection</a>&lt;AC, C&gt;",1,["authzen_diesel_core::connection::DbConnection"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/connection/struct.TxCleanupError.html\" title=\"struct authzen_diesel_core::connection::TxCleanupError\">TxCleanupError</a>",1,["authzen_diesel_core::connection::TxCleanupError"]],["impl&lt;'a, C&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_diesel_core/connection/enum.PooledConnection.html\" title=\"enum authzen_diesel_core::connection::PooledConnection\">PooledConnection</a>&lt;'a, C&gt;<span class=\"where fmt-newline\">where\n    C: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_diesel_core::connection::PooledConnection"]],["impl&lt;'query, Q, S&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/is_deleted/struct.IsDeletedFilter.html\" title=\"struct authzen_diesel_core::is_deleted::IsDeletedFilter\">IsDeletedFilter</a>&lt;'query, Q, S&gt;<span class=\"where fmt-newline\">where\n    S: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Q as FilterDsl&lt;Grouped&lt;IsNotNull&lt;&lt;S as <a class=\"trait\" href=\"authzen_diesel_core/deletable/trait.SoftDeletable.html\" title=\"trait authzen_diesel_core::deletable::SoftDeletable\">SoftDeletable</a>&gt;::<a class=\"associatedtype\" href=\"authzen_diesel_core/deletable/trait.SoftDeletable.html#associatedtype.DeletedAt\" title=\"type authzen_diesel_core::deletable::SoftDeletable::DeletedAt\">DeletedAt</a>&gt;&gt;&gt;&gt;::Output: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_diesel_core::is_deleted::IsDeletedFilter"]],["impl&lt;'query, Q, S&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/is_deleted/struct.IsNotDeletedFilter.html\" title=\"struct authzen_diesel_core::is_deleted::IsNotDeletedFilter\">IsNotDeletedFilter</a>&lt;'query, Q, S&gt;<span class=\"where fmt-newline\">where\n    S: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    &lt;Q as FilterDsl&lt;Grouped&lt;IsNull&lt;&lt;S as <a class=\"trait\" href=\"authzen_diesel_core/deletable/trait.SoftDeletable.html\" title=\"trait authzen_diesel_core::deletable::SoftDeletable\">SoftDeletable</a>&gt;::<a class=\"associatedtype\" href=\"authzen_diesel_core/deletable/trait.SoftDeletable.html#associatedtype.DeletedAt\" title=\"type authzen_diesel_core::deletable::SoftDeletable::DeletedAt\">DeletedAt</a>&gt;&gt;&gt;&gt;::Output: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_diesel_core::is_deleted::IsNotDeletedFilter"]],["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/is_deleted/struct.Wrapper.html\" title=\"struct authzen_diesel_core::is_deleted::Wrapper\">Wrapper</a>&lt;T&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_diesel_core::is_deleted::Wrapper"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.MAX_PAGINATE_COUNT.html\" title=\"struct authzen_diesel_core::paginate::MAX_PAGINATE_COUNT\">MAX_PAGINATE_COUNT</a>",1,["authzen_diesel_core::paginate::MAX_PAGINATE_COUNT"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Page.html\" title=\"struct authzen_diesel_core::paginate::Page\">Page</a>",1,["authzen_diesel_core::paginate::Page"]],["impl&lt;T, Q&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Paginated.html\" title=\"struct authzen_diesel_core::paginate::Paginated\">Paginated</a>&lt;T, Q&gt;<span class=\"where fmt-newline\">where\n    Q: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_diesel_core::paginate::Paginated"]],["impl&lt;K&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Paged.html\" title=\"struct authzen_diesel_core::paginate::Paged\">Paged</a>&lt;K&gt;<span class=\"where fmt-newline\">where\n    K: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_diesel_core::paginate::Paged"]],["impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_diesel_core/prelude/enum.DbEntityError.html\" title=\"enum authzen_diesel_core::prelude::DbEntityError\">DbEntityError</a>&lt;E&gt;<span class=\"where fmt-newline\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_diesel_core::_operations::DbEntityError"]],["impl&lt;C&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_diesel_core/pool/enum.Pool.html\" title=\"enum authzen_diesel_core::pool::Pool\">Pool</a>&lt;C&gt;",1,["authzen_diesel_core::pool::Pool"]]],
"authzen_diesel_proc_macros_core":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.DynamicSchema.html\" title=\"struct authzen_diesel_proc_macros_core::DynamicSchema\">DynamicSchema</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::DynamicSchema"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.SchemaName.html\" title=\"struct authzen_diesel_proc_macros_core::SchemaName\">SchemaName</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::SchemaName"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.TableName.html\" title=\"struct authzen_diesel_proc_macros_core::TableName\">TableName</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::TableName"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.ColumnName.html\" title=\"struct authzen_diesel_proc_macros_core::ColumnName\">ColumnName</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::ColumnName"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.ColumnSqlType.html\" title=\"struct authzen_diesel_proc_macros_core::ColumnSqlType\">ColumnSqlType</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::ColumnSqlType"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.Schema.html\" title=\"struct authzen_diesel_proc_macros_core::Schema\">Schema</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::Schema"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.Table.html\" title=\"struct authzen_diesel_proc_macros_core::Table\">Table</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::Table"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.Column.html\" title=\"struct authzen_diesel_proc_macros_core::Column\">Column</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::Column"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_diesel_proc_macros_core/enum.ColumnValueKind.html\" title=\"enum authzen_diesel_proc_macros_core::ColumnValueKind\">ColumnValueKind</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::ColumnValueKind"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_diesel_proc_macros_core/struct.Joinable.html\" title=\"struct authzen_diesel_proc_macros_core::Joinable\">Joinable</a>",1,["authzen_diesel_proc_macros_core::dynamic_schema::Joinable"]]],
"authzen_opa_core":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPATxEntityFull.html\" title=\"struct authzen_opa_core::OPATxEntityFull\">OPATxEntityFull</a>",1,["authzen_opa_core::data::opa_mongodb::OPATxEntityFull"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_opa_core/enum.DisableAuthorization.html\" title=\"enum authzen_opa_core::DisableAuthorization\">DisableAuthorization</a>",1,["authzen_opa_core::data::DisableAuthorization"]],["impl&lt;'a, T: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAType.html\" title=\"struct authzen_opa_core::OPAType\">OPAType</a>&lt;'a, T&gt;<span class=\"where fmt-newline\">where\n    &lt;T as <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html\" title=\"trait alloc::borrow::ToOwned\">ToOwned</a>&gt;::<a class=\"associatedtype\" href=\"https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html#associatedtype.Owned\" title=\"type alloc::borrow::ToOwned::Owned\">Owned</a>: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::data::OPAType"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.AccountSessionFields.html\" title=\"struct authzen_opa_core::AccountSessionFields\">AccountSessionFields</a>",1,["authzen_opa_core::data::AccountSessionFields"]],["impl&lt;V&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPATxEntity.html\" title=\"struct authzen_opa_core::OPATxEntity\">OPATxEntity</a>&lt;V&gt;<span class=\"where fmt-newline\">where\n    V: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::data::OPATxEntity"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetConfig.html\" title=\"struct authzen_opa_core::GetConfig\">GetConfig</a>",1,["authzen_opa_core::endpoints::config::GetConfig"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetConfigParams.html\" title=\"struct authzen_opa_core::GetConfigParams\">GetConfigParams</a>&lt;'a&gt;",1,["authzen_opa_core::endpoints::config::GetConfigParams"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetDocument.html\" title=\"struct authzen_opa_core::GetDocument\">GetDocument</a>",1,["authzen_opa_core::endpoints::data::GetDocument"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetDocumentParams.html\" title=\"struct authzen_opa_core::GetDocumentParams\">GetDocumentParams</a>&lt;'a&gt;",1,["authzen_opa_core::endpoints::data::GetDocumentParams"]],["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetDocumentResult.html\" title=\"struct authzen_opa_core::GetDocumentResult\">GetDocumentResult</a>&lt;T&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::endpoints::data::GetDocumentResult"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.UpsertDocument.html\" title=\"struct authzen_opa_core::UpsertDocument\">UpsertDocument</a>",1,["authzen_opa_core::endpoints::data::UpsertDocument"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.UpsertDocumentParams.html\" title=\"struct authzen_opa_core::UpsertDocumentParams\">UpsertDocumentParams</a>&lt;'a&gt;",1,["authzen_opa_core::endpoints::data::UpsertDocumentParams"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.PatchDocument.html\" title=\"struct authzen_opa_core::PatchDocument\">PatchDocument</a>",1,["authzen_opa_core::endpoints::data::PatchDocument"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.DeleteDocument.html\" title=\"struct authzen_opa_core::DeleteDocument\">DeleteDocument</a>",1,["authzen_opa_core::endpoints::data::DeleteDocument"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.DeleteDocumentParams.html\" title=\"struct authzen_opa_core::DeleteDocumentParams\">DeleteDocumentParams</a>&lt;'a&gt;",1,["authzen_opa_core::endpoints::data::DeleteDocumentParams"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.Health.html\" title=\"struct authzen_opa_core::Health\">Health</a>",1,["authzen_opa_core::endpoints::health::Health"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.HealthParams.html\" title=\"struct authzen_opa_core::HealthParams\">HealthParams</a>&lt;'a&gt;",1,["authzen_opa_core::endpoints::health::HealthParams"]],["impl&lt;'a, Data&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQuery.html\" title=\"struct authzen_opa_core::OPAQuery\">OPAQuery</a>&lt;'a, Data&gt;<span class=\"where fmt-newline\">where\n    Data: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::endpoints::query::OPAQuery"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQueryConfig.html\" title=\"struct authzen_opa_core::OPAQueryConfig\">OPAQueryConfig</a>&lt;'a&gt;",1,["authzen_opa_core::endpoints::query::OPAQueryConfig"]],["impl&lt;'a, Data&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQueryInput.html\" title=\"struct authzen_opa_core::OPAQueryInput\">OPAQueryInput</a>&lt;'a, Data&gt;<span class=\"where fmt-newline\">where\n    Data: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::endpoints::query::OPAQueryInput"]],["impl&lt;'a, Data&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_opa_core/enum.OPAQueryInputAction.html\" title=\"enum authzen_opa_core::OPAQueryInputAction\">OPAQueryInputAction</a>&lt;'a, Data&gt;<span class=\"where fmt-newline\">where\n    Data: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::endpoints::query::OPAQueryInputAction"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQueryResult.html\" title=\"struct authzen_opa_core::OPAQueryResult\">OPAQueryResult</a>",1,["authzen_opa_core::endpoints::query::OPAQueryResult"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetStatus.html\" title=\"struct authzen_opa_core::GetStatus\">GetStatus</a>",1,["authzen_opa_core::endpoints::status::GetStatus"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetStatusParams.html\" title=\"struct authzen_opa_core::GetStatusParams\">GetStatusParams</a>&lt;'a&gt;",1,["authzen_opa_core::endpoints::status::GetStatusParams"]],["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAResult.html\" title=\"struct authzen_opa_core::OPAResult\">OPAResult</a>&lt;T&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::models::OPAResult"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_opa_core/enum.OPAPolicyASTNode.html\" title=\"enum authzen_opa_core::OPAPolicyASTNode\">OPAPolicyASTNode</a>&lt;'a&gt;",1,["authzen_opa_core::models::OPAPolicyASTNode"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAPolicyASTNodeRef.html\" title=\"struct authzen_opa_core::OPAPolicyASTNodeRef\">OPAPolicyASTNodeRef</a>&lt;'a&gt;",1,["authzen_opa_core::models::OPAPolicyASTNodeRef"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_opa_core/enum.OPAPolicyPathNode.html\" title=\"enum authzen_opa_core::OPAPolicyPathNode\">OPAPolicyPathNode</a>&lt;'a&gt;",1,["authzen_opa_core::models::OPAPolicyPathNode"]],["impl&lt;'a, 'b&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAPolicyASTNodeRefRef.html\" title=\"struct authzen_opa_core::OPAPolicyASTNodeRefRef\">OPAPolicyASTNodeRefRef</a>&lt;'a, 'b&gt;<span class=\"where fmt-newline\">where\n    'a: 'b,</span>",1,["authzen_opa_core::models::OPAPolicyASTNodeRefRef"]],["impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_opa_core/enum.CowSlice.html\" title=\"enum authzen_opa_core::CowSlice\">CowSlice</a>&lt;'a, T&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_opa_core::models::CowSlice"]],["impl&lt;Connector&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAClient.html\" title=\"struct authzen_opa_core::OPAClient\">OPAClient</a>&lt;Connector&gt;",1,["authzen_opa_core::OPAClient"]]],
"authzen_opa_proc_macros_core":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_proc_macros_core/struct.OPAContextAccountSessionAttributeArgs.html\" title=\"struct authzen_opa_proc_macros_core::OPAContextAccountSessionAttributeArgs\">OPAContextAccountSessionAttributeArgs</a>",1,["authzen_opa_proc_macros_core::OPAContextAccountSessionAttributeArgs"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_opa_proc_macros_core/struct.OPAContextAccountSessionAttributeArg.html\" title=\"struct authzen_opa_proc_macros_core::OPAContextAccountSessionAttributeArg\">OPAContextAccountSessionAttributeArg</a>",1,["authzen_opa_proc_macros_core::OPAContextAccountSessionAttributeArg"]]],
"authzen_proc_macro_util":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_proc_macro_util/struct.UnmatchedPathPrefix.html\" title=\"struct authzen_proc_macro_util::UnmatchedPathPrefix\">UnmatchedPathPrefix</a>",1,["authzen_proc_macro_util::match_path::UnmatchedPathPrefix"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_proc_macro_util/struct.MatchPathError.html\" title=\"struct authzen_proc_macro_util::MatchPathError\">MatchPathError</a>",1,["authzen_proc_macro_util::match_path::MatchPathError"]],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_proc_macro_util/struct.MatchedAttribute.html\" title=\"struct authzen_proc_macro_util::MatchedAttribute\">MatchedAttribute</a>&lt;'a&gt;",1,["authzen_proc_macro_util::MatchedAttribute"]]],
"authzen_proc_macros_core":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_proc_macros_core/struct.ActionArgs.html\" title=\"struct authzen_proc_macros_core::ActionArgs\">ActionArgs</a>",1,["authzen_proc_macros_core::action::ActionArgs"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_proc_macros_core/struct.AuthzObjectArgs.html\" title=\"struct authzen_proc_macros_core::AuthzObjectArgs\">AuthzObjectArgs</a>",1,["authzen_proc_macros_core::authz_object::AuthzObjectArgs"]]],
"authzen_service_util":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_service_util/enum.EnvError.html\" title=\"enum authzen_service_util::EnvError\">EnvError</a>",1,["authzen_service_util::env::EnvError"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_service_util/enum.DbError.html\" title=\"enum authzen_service_util::DbError\">DbError</a>",1,["authzen_service_util::error::db::DbError"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.Error.html\" title=\"struct authzen_service_util::Error\">Error</a>",1,["authzen_service_util::error::Error"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_service_util/enum.BaseClientError.html\" title=\"enum authzen_service_util::BaseClientError\">BaseClientError</a>",1,["authzen_service_util::client::BaseClientError"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.Path.html\" title=\"struct authzen_service_util::Path\">Path</a>",1,["authzen_service_util::client::Path"]],["impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.Optional.html\" title=\"struct authzen_service_util::Optional\">Optional</a>&lt;E&gt;<span class=\"where fmt-newline\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_service_util::client::Optional"]],["impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.Ignore.html\" title=\"struct authzen_service_util::Ignore\">Ignore</a>&lt;E&gt;<span class=\"where fmt-newline\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_service_util::client::Ignore"]],["impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.Raw.html\" title=\"struct authzen_service_util::Raw\">Raw</a>&lt;E&gt;<span class=\"where fmt-newline\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_service_util::client::Raw"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_service_util/enum.NextPage.html\" title=\"enum authzen_service_util::NextPage\">NextPage</a>",1,["authzen_service_util::client::NextPage"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_service_util/enum.Pagination.html\" title=\"enum authzen_service_util::Pagination\">Pagination</a>",1,["authzen_service_util::client::Pagination"]],["impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.Paged.html\" title=\"struct authzen_service_util::Paged\">Paged</a>&lt;E&gt;<span class=\"where fmt-newline\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_service_util::client::Paged"]],["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.DefaultResponse.html\" title=\"struct authzen_service_util::DefaultResponse\">DefaultResponse</a>&lt;T&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_service_util::client::DefaultResponse"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.ApiPage.html\" title=\"struct authzen_service_util::ApiPage\">ApiPage</a>",1,["authzen_service_util::paginate::ApiPage"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_service_util/enum.ApiPageError.html\" title=\"enum authzen_service_util::ApiPageError\">ApiPageError</a>",1,["authzen_service_util::paginate::ApiPageError"]],["impl&lt;B&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"enum\" href=\"authzen_service_util/enum.RawBody.html\" title=\"enum authzen_service_util::RawBody\">RawBody</a>&lt;B&gt;<span class=\"where fmt-newline\">where\n    B: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</span>",1,["authzen_service_util::server::RawBody"]],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a> for <a class=\"struct\" href=\"authzen_service_util/struct.RequestId.html\" title=\"struct authzen_service_util::RequestId\">RequestId</a>",1,["authzen_service_util::server::RequestId"]]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()