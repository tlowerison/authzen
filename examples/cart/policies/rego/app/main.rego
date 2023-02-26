package app

import data.util.fetch
import future.keywords.every
import future.keywords.in

# input
action := input.action

subject := subject {
	token := io.jwt.decode_verify(input.token, {
		"alg": opa.runtime().env.JWT_ALGORITHM,
		"cert": replace(opa.runtime().env.JWT_PUBLIC_CERTIFICATE, "_", "\n"),
	})
	is_verified := token[0]
	is_verified == true
	subject := token[2].state
}

# authz is the binary policy decision output
default authz := false

default deny := {}

default allow := {}

# denials are enforced first to minimize execution time of unsound queries
authz {
	count(deny) == 0
	count(allow) > 0
}

# reasons are aggregated messages from the individual rules which are the source
# of either first any denials and then if there are no denials, any allowances
# ordering of these three declarations matters
default reasons := {}

reasons := deny {
	count(deny) > 0
}

reasons := allow {
	count(deny) == 0
	count(allow) > 0
}

# consts
create := "create"

delete := "delete"

read := "read"

update := "update"

root_role := role {
	roles := fetch({
		"service": data.app.accounts.service,
		"entity": data.app.accounts.role.entity,
	})
	role := roles[_]
	role.title == "root"
}

service_role := role {
	roles := fetch({
		"service": data.app.accounts.service,
		"entity": data.app.accounts.role.entity,
	})
	role := roles[_]
	role.title == "service"
}

is_root {
	some role_id in subject.role_ids
	role_id == root_role.id
}

is_service {
	some role_id in subject.role_ids
	role_id == service_role.id
}

deny := {"input is not sound"} {
	not is_input_sound
}

is_input_sound {
	not subject
	data.app.action.type
	data.app.action.object.service
	data.app.action.object.entity
	is_action_sound
}

is_input_sound {
	subject.account_id
	subject.role_ids
	data.app.action.type
	data.app.action.object.service
	data.app.action.object.entity
	is_action_sound
}

is_action_sound {
	data.app.action.type == create
	data.app.action.object.records
}

is_action_sound {
	data.app.action.type == delete
	data.app.action.object.ids
}

is_action_sound {
	data.app.action.type == read
	data.app.action.object.ids
}

is_action_sound {
	data.app.action.type == update
	data.app.action.object.patches
}

# service-based denial policies are then enforced
deny := data.app.accounts.deny {
	is_input_sound
	action.object.service == data.app.accounts.service
	count(data.app.accounts.deny) > 0
}

deny := data.app.events.deny {
	is_input_sound
	action.object.service == data.app.events.service
	count(data.app.events.deny) > 0
}

# service-based allowance policies are then enforced
# note that these policies only check that at least one reason
# is provided for an allowance of acting on potentially more than
# one object -- it is up to the source allowance rules to determine
# whether all objects being acted have allowances
# this is done for three reasons:
# - service-specific admins should have an easy escape hatch for allowances
#     (i.e. they shouldn't have to provide a reason for every object they're acting on)
# - this keeps tests in the lowest level packages fully testable
#     (i.e. they should still reject an input whose action is allowed for some but not all of its objects)
# - if there duplicate object ids/records/patches, trying to check that all
#   have allowances at this level is more difficult than deeper in the eval tree
allow := {"root"} {
	is_root
}

allow := {"no-op"} {
	not is_root
	data.app.action.type == create
	count(data.app.action.object.records) == 0
}

allow := {"no-op"} {
	not is_root
	data.app.action.type == delete
	count(data.app.action.object.ids) == 0
}

allow := {"no-op"} {
	not is_root
	data.app.action.type == read
	count(data.app.action.object.ids) == 0
}

allow := {"no-op"} {
	not is_root
	data.app.action.type == update
	count(data.app.action.object.patches) == 0
}

allow := data.app.accounts.allow {
	not is_root
	action.object.service == data.app.accounts.service
	count(data.app.accounts.allow) > 0
}

allow := data.app.events.allow {
	not is_root
	action.object.service == data.app.events.service
	count(data.app.events.allow) > 0
}
