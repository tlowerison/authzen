package app.examples_cart.item

type := "item"

default deny := {}

default allow := {}

deny := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.item.create.deny
}

allow := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.item.create.allow
}

deny := reasons {
	data.app.event.action == data.app.delete
	reasons := data.app.examples_cart.item.delete.deny
}

allow := reasons {
	data.app.event.action == data.app.delete
	reasons := data.app.examples_cart.item.delete.allow
}

deny := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.item.read.deny
}

allow := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.item.read.allow
}

deny := reasons {
	data.app.event.action == data.app.update
	reasons := data.app.examples_cart.item.update.deny
}

allow := reasons {
	data.app.event.action == data.app.update
	reasons := data.app.examples_cart.item.update.allow
}
