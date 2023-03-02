package app.examples_cart

import data.util.fetch
import future.keywords

# consts

service := "examples_cart"

# policies

default deny := {}

default allow := {}

deny := reasons if {
	data.app.event.object.type == data.app.examples_cart.account.type
	reasons := data.app.examples_cart.account.deny
}

allow := reasons if {
	data.app.event.object.type == data.app.examples_cart.account.type
	reasons := data.app.examples_cart.account.allow
}

deny := reasons if {
	data.app.event.object.type == data.app.examples_cart.cart.type
	reasons := data.app.examples_cart.cart.deny
}

allow := reasons if {
	data.app.event.object.type == data.app.examples_cart.cart.type
	reasons := data.app.examples_cart.cart.allow
}

deny := reasons if {
	data.app.event.object.type == data.app.examples_cart.cart_item.type
	reasons := data.app.examples_cart.cart_item.deny
}

allow := reasons if {
	data.app.event.object.type == data.app.examples_cart.cart_item.type
	reasons := data.app.examples_cart.cart_item.allow
}

deny := reasons if {
	data.app.event.object.type == data.app.examples_cart.item.type
	reasons := data.app.examples_cart.item.deny
}

allow := reasons if {
	data.app.event.object.type == data.app.examples_cart.item.type
	reasons := data.app.examples_cart.item.allow
}
