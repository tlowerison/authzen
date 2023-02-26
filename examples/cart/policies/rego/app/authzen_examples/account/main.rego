package app.examples_cart.account

entity := "account"

default deny := {}

default allow := {}

deny := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.account.create.deny
}

allow := reasons {
	data.app.action.type == data.app.create
	reasons := data.app.examples_cart.account.create.allow
}

deny := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.account.delete.deny
}

allow := reasons {
	data.app.action.type == data.app.delete
	reasons := data.app.examples_cart.account.delete.allow
}

deny := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.account.read.deny
}

allow := reasons {
	data.app.action.type == data.app.read
	reasons := data.app.examples_cart.account.read.allow
}

deny := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.account.update.deny
}

allow := reasons {
	data.app.action.type == data.app.update
	reasons := data.app.examples_cart.account.update.allow
}
