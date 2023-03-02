package app

import data.util.fetch
import future.keywords.every
import future.keywords.in

# input
subject := subject {
	token := io.jwt.decode_verify(input.subject.value.token, {
		"alg": opa.runtime().env.JWT_ALGORITHM,
		"cert": replace(opa.runtime().env.JWT_PUBLIC_CERTIFICATE, "_", "\n"),
	})
	is_verified := token[0]
	is_verified == true
	subject := token[2].state
}

event := event {
	subject
	event := {
		"subject": subject,
		"action": input.action,
		"object": input.object,
		"input": input.input,
		"context": input.context,
	}
}

event := event {
	not subject
	event := {
		"subject": null,
		"action": input.action,
		"object": input.object,
		"input": input.input,
		"context": input.context,
	}
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

deny := {"event is unsound"} {
	not is_event_sound
}

is_event_sound {
	is_subject_sound
	is_action_sound
	is_object_sound
}

is_subject_sound {
	event.subject == null
}

is_subject_sound {
	event.subject != null
	event.subject.account_id
}

is_action_sound {
	event.action == create
}

is_action_sound {
	event.action == delete
}

is_action_sound {
	event.action == read
}

is_action_sound {
	event.action == update
}

is_object_sound {
	event.object.service
	event.object.type
}

# service-based denial policies are then enforced
deny := data.app.examples_cart.deny {
	is_event_sound
	event.object.service == data.app.examples_cart.service
	count(data.app.examples_cart.deny) > 0
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
allow := {"no-op"} {
	event.action == create
	count(event.object.records) == 0
}

allow := {"no-op"} {
	event.action == delete
	count(event.object.ids) == 0
}

allow := {"no-op"} {
	event.action == read
	count(event.object.ids) == 0
}

allow := {"no-op"} {
	event.action == update
	count(event.object.patches) == 0
}

allow := data.app.examples_cart.allow {
	event.object.service == data.app.examples_cart.service
	count(data.app.examples_cart.allow) > 0
}
