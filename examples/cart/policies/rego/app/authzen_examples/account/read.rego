package app.examples_cart.account.read

import data.util.fetch
import future.keywords

default deny := {}

default allow := {}

subject := data.app.subject

ids := data.app.action.object.ids

allow := allow_read if {
	every id in ids {
		allow_read[id]
	}
}

allow_read[id] := reason if {
	id := ids[_]
	id == subject.account_id
	reason := "subject can read itself"
}
