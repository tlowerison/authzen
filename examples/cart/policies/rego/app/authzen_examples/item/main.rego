package app.examples_cart.item

entity := "item"

default deny := {}

default allow := {}

deny := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.item.create.deny
}

allow := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.item.create.allow
}

deny := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.item.delete.deny
}

allow := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.item.delete.allow
}

deny := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.item.read.deny
}

allow := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.item.read.allow
}

deny := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.item.update.deny
}

allow := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.item.update.allow
}
