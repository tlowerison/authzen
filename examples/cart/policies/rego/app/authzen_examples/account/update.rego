package app.examples_cart.account.update

import future.keywords

default deny := {}

default allow := {}

subject := data.app.subject

patches := data.app.action.object.patches

allow := allow_update if {
	every patch in patches {
		allow_update[patch.id]
	}
}

allow_update[id] := reason if {
	patch := patches[_]
	id := patch.id
	patch.id == subject.account_id
	reason := "subject can update itself"
}
