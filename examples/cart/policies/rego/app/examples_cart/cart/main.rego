package app.examples_cart.cart

type := "cart"

default deny := {}

default allow := {}

deny := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.cart.create.deny
}

allow := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.cart.create.allow
}

deny := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.cart.read.deny
}

allow := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.cart.read.allow
}
