package app.examples_cart.cart_item.delete

import future.keywords

default deny := {}

default allow := {}

subject := data.app.subject

ids := data.app.event.object.ids

allow := allow_delete if {
	every id in ids {
		allow_delete[id]
	}
}

allow_delete[id] := reason if {
	id := ids[_]
	id == subject.cart_item_id
	reason := "subject can delete itself"
}
