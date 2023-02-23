(function() {var implementors = {
"authzen_core":[["impl&lt;T, Id&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_core/struct.TxCacheEntity.html\" title=\"struct authzen_core::TxCacheEntity\">TxCacheEntity</a>&lt;T, Id&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,\n    Id: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</span>"],["impl&lt;Subject, Action, Object, Input, Context&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_core/struct.Event.html\" title=\"struct authzen_core::Event\">Event</a>&lt;Subject, Action, Object, Input, Context&gt;<span class=\"where fmt-newline\">where\n    Subject: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,\n    Action: <a class=\"trait\" href=\"authzen_core/trait.ActionType.html\" title=\"trait authzen_core::ActionType\">ActionType</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,\n    Object: <a class=\"trait\" href=\"authzen_core/trait.ObjectType.html\" title=\"trait authzen_core::ObjectType\">ObjectType</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,\n    Input: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,\n    Context: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</span>"]],
"authzen_diesel_core":[["impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_diesel_core/paginate/struct.Page.html\" title=\"struct authzen_diesel_core::paginate::Page\">Page</a>"]],
"authzen_opa_core":[["impl&lt;'a, Data&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQuery.html\" title=\"struct authzen_opa_core::OPAQuery\">OPAQuery</a>&lt;'a, Data&gt;<span class=\"where fmt-newline\">where\n    Data: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,</span>"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetDocumentParams.html\" title=\"struct authzen_opa_core::GetDocumentParams\">GetDocumentParams</a>&lt;'a&gt;"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetConfigParams.html\" title=\"struct authzen_opa_core::GetConfigParams\">GetConfigParams</a>&lt;'a&gt;"],["impl&lt;'a, Data&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQueryInput.html\" title=\"struct authzen_opa_core::OPAQueryInput\">OPAQueryInput</a>&lt;'a, Data&gt;<span class=\"where fmt-newline\">where\n    Data: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</span>"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.UpsertDocumentParams.html\" title=\"struct authzen_opa_core::UpsertDocumentParams\">UpsertDocumentParams</a>&lt;'a&gt;"],["impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQueryResult.html\" title=\"struct authzen_opa_core::OPAQueryResult\">OPAQueryResult</a>"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.DeleteDocumentParams.html\" title=\"struct authzen_opa_core::DeleteDocumentParams\">DeleteDocumentParams</a>&lt;'a&gt;"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAQueryConfig.html\" title=\"struct authzen_opa_core::OPAQueryConfig\">OPAQueryConfig</a>&lt;'a&gt;"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.GetStatusParams.html\" title=\"struct authzen_opa_core::GetStatusParams\">GetStatusParams</a>&lt;'a&gt;"],["impl&lt;'a, Data&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"enum\" href=\"authzen_opa_core/enum.OPAQueryInputAction.html\" title=\"enum authzen_opa_core::OPAQueryInputAction\">OPAQueryInputAction</a>&lt;'a, Data&gt;<span class=\"where fmt-newline\">where\n    Data: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</span>"],["impl&lt;V&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPATxEntity.html\" title=\"struct authzen_opa_core::OPATxEntity\">OPATxEntity</a>&lt;V&gt;<span class=\"where fmt-newline\">where\n    V: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</span>"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.HealthParams.html\" title=\"struct authzen_opa_core::HealthParams\">HealthParams</a>&lt;'a&gt;"],["impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.AccountSessionFields.html\" title=\"struct authzen_opa_core::AccountSessionFields\">AccountSessionFields</a>"],["impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"authzen_opa_core/struct.OPAType.html\" title=\"struct authzen_opa_core::OPAType\">OPAType</a>&lt;'a, T&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/alloc/borrow/trait.ToOwned.html\" title=\"trait alloc::borrow::ToOwned\">ToOwned</a> + 'a + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</span>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()