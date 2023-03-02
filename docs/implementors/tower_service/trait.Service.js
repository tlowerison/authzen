(function() {var implementors = {
"authzen_session":[["impl&lt;ReqBody, ResBody, I, P, S, K, V, R&gt; Service&lt;<a class=\"struct\" href=\"https://docs.rs/http/0.2.8/http/request/struct.Request.html\" title=\"struct http::request::Request\">Request</a>&lt;ReqBody&gt;&gt; for <a class=\"struct\" href=\"authzen_session/struct.SessionService.html\" title=\"struct authzen_session::SessionService\">SessionService</a>&lt;I, P, S, K, V&gt;<span class=\"where fmt-newline\">where\n    I: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + Service&lt;<a class=\"struct\" href=\"https://docs.rs/http/0.2.8/http/request/struct.Request.html\" title=\"struct http::request::Request\">Request</a>&lt;ReqBody&gt;, Response = <a class=\"struct\" href=\"https://docs.rs/http/0.2.8/http/response/struct.Response.html\" title=\"struct http::response::Response\">Response</a>&lt;ResBody&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static,\n    &lt;I as Service&lt;<a class=\"struct\" href=\"https://docs.rs/http/0.2.8/http/request/struct.Request.html\" title=\"struct http::request::Request\">Request</a>&lt;ReqBody&gt;&gt;&gt;::Future: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,\n    S: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"authzen_session/trait.SessionStore.html\" title=\"trait authzen_session::SessionStore\">SessionStore</a>&lt;Value = R&gt;,\n    K: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static,\n    V: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static,\n    R: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a> + <a class=\"trait\" href=\"https://docs.rs/serde/1.0.152/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> + <a class=\"trait\" href=\"authzen_session/trait.SessionValue.html\" title=\"trait authzen_session::SessionValue\">SessionValue</a>&lt;ReqBody, S&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + 'static,\n    P: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + 'static,\n    <a class=\"struct\" href=\"authzen_session/struct.Session.html\" title=\"struct authzen_session::Session\">Session</a>&lt;R&gt;: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"authzen_session/trait.RawSession.html\" title=\"trait authzen_session::RawSession\">RawSession</a>&lt;P, Key = K, Validation = V&gt;,\n    ReqBody: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + 'static,\n    ResBody: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static,</span>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()