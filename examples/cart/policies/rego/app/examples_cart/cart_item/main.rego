package app.examples_cart.cart_item

type := "cart_item"

default deny := {}

default allow := {}

deny := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.cart_item.create.deny
}

allow := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.cart_item.create.allow
}

deny := reasons {
	data.app.event.action == data.app.delete
	reasons := data.app.examples_cart.cart_item.delete.deny
}

allow := reasons {
	data.app.event.action == data.app.delete
	reasons := data.app.examples_cart.cart_item.delete.allow
}

deny := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.cart_item.read.deny
}

allow := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.cart_item.read.allow
}
