package app.examples_cart.cart.read

import data.util.fetch
import future.keywords

default deny := {}

default allow := {}

subject := data.app.subject

ids := data.app.event.object.ids

allow := allow_read if {
	every id in ids {
		allow_read[id]
	}
}

allow_read[id] := reason if {
	id := ids[_]

	carts := data.util.fetch({
		"service": data.app.examples_cart.service,
		"type": data.app.examples_cart.cart.type,
		"ids": {id | id := ids[_]},
	})
	cart := carts[id]

	cart.account_id == subject.account_id

	reason := "subject can read its own cart"
}
