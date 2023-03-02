package app.examples_cart.item.create

import future.keywords

default deny := {}

default allow := {}

subject := data.app.subject

posts := data.app.event.input

allowed_usernames := ["super_special_admin_username"]

allow := allow_create if {
	every post in posts {
		allow_create[post.id]
	}
}

allow_create[id] := reason if {
	id := posts[_].id

	# arbitrary rule for who can create items based on usernames
	accounts := data.util.fetch({
		"service": data.app.examples_cart.service,
		"type": data.app.examples_cart.account.type,
		"id": subject.account_id,
	})
	subject_account := accounts[subject.account_id]

	subject_account.username in allowed_usernames

	reason := "subject has an authorized username"
}
