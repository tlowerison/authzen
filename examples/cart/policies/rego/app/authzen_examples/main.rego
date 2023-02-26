package app.examples_cart

import data.util.fetch
import future.keywords

# consts

service := "examples_cart"

# policies

default deny := {}

default allow := {}

deny := reasons if {
	data.app.action.object.entity == data.app.examples_cart.account.entity
	reasons := data.app.examples_cart.account.deny
}

allow := reasons if {
	data.app.action.object.entity == data.app.examples_cart.account.entity
	reasons := data.app.examples_cart.account.allow
}

deny := reasons if {
	data.app.action.object.entity == data.app.examples_cart.cart.entity
	reasons := data.app.examples_cart.cart.deny
}

allow := reasons if {
	data.app.action.object.entity == data.app.examples_cart.cart.entity
	reasons := data.app.examples_cart.cart.allow
}

deny := reasons if {
	data.app.action.object.entity == data.app.examples_cart.cart_item.entity
	reasons := data.app.examples_cart.cart_item.deny
}

allow := reasons if {
	data.app.action.object.entity == data.app.examples_cart.cart_item.entity
	reasons := data.app.examples_cart.cart_item.allow
}

deny := reasons if {
	data.app.action.object.entity == data.app.examples_cart.item.entity
	reasons := data.app.examples_cart.item.deny
}

allow := reasons if {
	data.app.action.object.entity == data.app.examples_cart.item.entity
	reasons := data.app.examples_cart.item.allow
}
