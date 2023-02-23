(function() {var implementors = {
"authzen_diesel_core":[["impl&lt;AC, C&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;Mutex&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn for&lt;'r&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/ops/function/trait.FnOnce.html\" title=\"trait core::ops::function::FnOnce\">FnOnce</a>(&amp;'r <a class=\"struct\" href=\"authzen_diesel_core/connection/struct.DbConnection.html\" title=\"struct authzen_diesel_core::connection::DbConnection\">DbConnection</a>&lt;AC, <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;'r mut AC</a>&gt;) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"authzen_diesel_core/connection/struct.TxCleanupError.html\" title=\"struct authzen_diesel_core::connection::TxCleanupError\">TxCleanupError</a>&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'r, <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/alloc/struct.Global.html\" title=\"struct alloc::alloc::Global\">Global</a>&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + 'static, <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/alloc/struct.Global.html\" title=\"struct alloc::alloc::Global\">Global</a>&gt;, <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/alloc/struct.Global.html\" title=\"struct alloc::alloc::Global\">Global</a>&gt;&gt;&gt;&gt; for <a class=\"struct\" href=\"authzen_diesel_core/connection/struct.DbConnection.html\" title=\"struct authzen_diesel_core::connection::DbConnection\">DbConnection</a>&lt;AC, C&gt;"],["impl&lt;AC, C&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://docs.rs/uuid/1.3.0/uuid/struct.Uuid.html\" title=\"struct uuid::Uuid\">Uuid</a>&gt;&gt; for <a class=\"struct\" href=\"authzen_diesel_core/connection/struct.DbConnection.html\" title=\"struct authzen_diesel_core::connection::DbConnection\">DbConnection</a>&lt;AC, C&gt;"],["impl&lt;K&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Page.html\" title=\"struct authzen_diesel_core::paginate::Page\">Page</a>&gt; for <a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Paged.html\" title=\"struct authzen_diesel_core::paginate::Paged\">Paged</a>&lt;K&gt;"],["impl&lt;AC, C&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;C&gt;&gt;&gt; for <a class=\"struct\" href=\"authzen_diesel_core/connection/struct.DbConnection.html\" title=\"struct authzen_diesel_core::connection::DbConnection\">DbConnection</a>&lt;AC, C&gt;"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Page.html\" title=\"struct authzen_diesel_core::paginate::Page\">Page</a>&gt; for <a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Page.html\" title=\"struct authzen_diesel_core::paginate::Page\">Page</a>"]],
"authzen_opa_core":[["impl&lt;'a, T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html\" title=\"trait alloc::borrow::ToOwned\">ToOwned</a> + 'a + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/alloc/borrow/enum.Cow.html\" title=\"enum alloc::borrow::Cow\">Cow</a>&lt;'a, T&gt;&gt; for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAType.html\" title=\"struct authzen_opa_core::OPAType\">OPAType</a>&lt;'a, T&gt;"]],
"authzen_proc_macro_util":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://docs.rs/syn/1.0.107/syn/path/struct.Path.html\" title=\"struct syn::path::Path\">Path</a>&gt;&gt; for <a class=\"struct\" href=\"authzen_proc_macro_util/struct.UnmatchedPathPrefix.html\" title=\"struct authzen_proc_macro_util::UnmatchedPathPrefix\">UnmatchedPathPrefix</a>"]],
"authzen_service_util":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.u8.html\">u8</a>]&gt; for <a class=\"struct\" href=\"authzen_service_util/struct.Path.html\" title=\"struct authzen_service_util::Path\">Path</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.str.html\">str</a>&gt; for <a class=\"struct\" href=\"authzen_service_util/struct.Path.html\" title=\"struct authzen_service_util::Path\">Path</a>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()