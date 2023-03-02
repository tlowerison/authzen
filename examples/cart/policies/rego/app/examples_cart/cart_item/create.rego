package app.examples_cart.cart_item.create

import future.keywords

default deny := {}

default allow := {}

subject := data.app.subject

posts := data.app.event.input

allow := allow_create if {
	every post in posts {
		allow_create[post.id]
	}
}

allow_create[id] := reason if {
	post := posts[_]
	id := post.id

	carts := data.util.fetch({
		"service": data.app.examples_cart.service,
		"type": data.app.examples_cart.cart.type,
		"ids": {cart_id | cart_id := posts[_].cart_id},
	})
	cart := carts[post.cart_id]

	cart.account_id == subject.account_id

	reason := "subject can add items to their own cart"
}
