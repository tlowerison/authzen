package app.examples_cart.account.delete

import future.keywords

default deny := {}

default allow := {}

subject := data.app.subject

ids := data.app.action.object.ids

allow := allow_delete if {
	every id in ids {
		allow_delete[id]
	}
}

allow_delete[id] := reason if {
	id := ids[_]
	id == subject.account_id
	reason := "subject can delete itself"
}
