package util

import future.keywords

# NOTE: The first two implementations of fetch are for testing only
# they effectively just return mocks which should be set under the path data.external.
# As of now, these mocks are not filtered the way the policy information point handles
# filtering by key (i.e. get events.event where events.event.organization_id == x), instead it just returns
# all entities set under the path provided in the request (i.e. returns all of data.external.events.event).

# if USE_POLICY_INFORMATION_POINT is not set, all calls to fetch should use data.external
fetch(body) := values if {
	not opa.runtime().env.USE_POLICY_INFORMATION_POINT
	trace(sprintf("fetch: %v", [body]))
	values := body_filter(body, object.get(data.external, [body.service, body.entity], {}))
	trace(sprintf("values: %v", [values]))
}

# if USE_POLICY_INFORMATION_POINT is not set to true, all calls to fetch should use data.external
fetch(body) := values if {
	opa.runtime().env.USE_POLICY_INFORMATION_POINT != "true"
	trace(sprintf("fetch: %v", [body]))
	values := body_filter(body, object.get(data.external, [body.service, body.entity], {}))
	trace(sprintf("values: %v", [values]))
}

# all headers must be non-null so there is are headers implementations for when transaction_id exists / does not exist
#
# transaction_id: if provided, is forwarded to the policy information point
# the policy information point may use this to check an external cache for data
# local to the ongoing transaction which this OPA request is contained in
# essentially, this is necessary because of the isolation property of ACID databases:
# if transactions have an isolation of read complete or stricter, OPA and its policy point
# won't be able to use uncommitted but queried data mutations in its decision making and
# that could lead to a collapse in a policy decision of something like a nested insertion
# where the children to be inserted require the inserted parent's data to make a correct
# policy decision
headers := headers if {
	is_string(input.transaction_id)
	headers := {
		"accept": "application/json",
		"authorization": sprintf("Bearer %s", [opa.runtime().env.POLICY_INFORMATION_POINT_TOKEN]),
		"content-type": "application/json",
		"x-transaction-id": input.transaction_id,
	}
}

headers := headers if {
	not is_string(input.transaction_id)
	headers := {
		"accept": "application/json",
		"authorization": sprintf("Bearer %s", [opa.runtime().env.POLICY_INFORMATION_POINT_TOKEN]),
		"content-type": "application/json",
	}
}

fetch(body) := values if {
	trace(sprintf("fetch: %v", [body]))
	values := http.send({
		"force_json_decode": true,
		"headers": headers(),
		"cache": true,
		"caching_mode": "deserialized",
		"method": "POST",
		"raise_error": true,
		"url": opa.runtime().env.POLICY_INFORMATION_POINT_URL,
		"body": body,
	}).body

	# policy information point is able to control whether/how long data is cached when this is true

	# policy information point uses POST method to avoid any size limitations of forcing requests into the url
	# pip requests should never mutate external data stores

	trace(sprintf("values: %v", [values]))
}

# simulates application of filters passed to the policy information point
# NOTE: this function relies on filters only using fields ending with
# either "id" or "ids"; it expects fields ending with "id" to have type
# Uuid and fields ending with "ids" to have type Vec<Uuid>
body_filter(body, values) := filtered if {
	filters := object.remove(body, ["service", "entity"])

	singular_id_filters := {key: value |
		key := object.keys(filters)[_]
		endswith(key, "id")
		value := {filters[key]}
	}
	plural_id_filters := {key: value |
		plural_key := object.keys(filters)[_]
		endswith(plural_key, "ids")
		key := substring(plural_key, 0, count(plural_key) - 1)
		value := filters[plural_key]
	}
	keys := object.keys(singular_id_filters) | object.keys(plural_id_filters)
	parsed_filters := {key: values |
		key := keys[_]
		singular_values := object.get(singular_id_filters, key, {})
		plural_values := object.get(plural_id_filters, key, {})
		values := {v | v := singular_values[_]} | {v | v := plural_values[_]}
	}

	filtered := {key: value |
		key := object.keys(values)[_]
		value := values[key]
		every filter_key, filter_values in parsed_filters {
			value[filter_key] in filter_values
		}
	}
}
