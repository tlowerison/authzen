package app.examples_cart.cart_item

entity := "cart_item"

default deny := {}

default allow := {}

deny := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.cart_item.create.deny
}

allow := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.cart_item.create.allow
}

deny := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.cart_item.delete.deny
}

allow := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.cart_item.delete.allow
}

deny := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.cart_item.read.deny
}

allow := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.cart_item.read.allow
}

deny := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.cart_item.update.deny
}

allow := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.cart_item.update.allow
}
