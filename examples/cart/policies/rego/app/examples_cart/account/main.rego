package app.examples_cart.account

type := "account"

default deny := {}

default allow := {}

deny := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.account.create.deny
}

allow := reasons {
	data.app.event.action == data.app.create
	reasons := data.app.examples_cart.account.create.allow
}

deny := reasons {
	data.app.event.action == data.app.delete
	reasons := data.app.examples_cart.account.delete.deny
}

allow := reasons {
	data.app.event.action == data.app.delete
	reasons := data.app.examples_cart.account.delete.allow
}

deny := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.account.read.deny
}

allow := reasons {
	data.app.event.action == data.app.read
	reasons := data.app.examples_cart.account.read.allow
}

deny := reasons {
	data.app.event.action == data.app.update
	reasons := data.app.examples_cart.account.update.deny
}

allow := reasons {
	data.app.event.action == data.app.update
	reasons := data.app.examples_cart.account.update.allow
}
