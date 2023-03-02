package app.examples_cart.cart_item.read

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

	cart_items := data.util.fetch({
		"service": data.app.examples_cart.service,
		"type": data.app.examples_cart.cart_item.type,
		"ids": {id | id := ids[_]},
	})
	cart_item := cart_items[id]

	carts := data.util.fetch({
		"service": data.app.examples_cart.service,
		"type": data.app.examples_cart.cart.type,
		"ids": {id | id := cart_items[_].cart_id},
	})
	cart := carts[cart_item.cart_id]

	cart.account_id == subject.account_id

	reason := "subject can read items from its own cart"
}
