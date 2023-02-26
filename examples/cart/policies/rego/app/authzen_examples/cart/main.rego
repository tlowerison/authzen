package app.examples_cart.cart

entity := "cart"

default deny := {}

default allow := {}

deny := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.cart.create.deny
}

allow := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.cart.create.allow
}

deny := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.cart.delete.deny
}

allow := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.cart.delete.allow
}

deny := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.cart.read.deny
}

allow := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.cart.read.allow
}

deny := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.cart.update.deny
}

allow := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.cart.update.allow
}
