#!/usr/bin/env bats

@test "Mutate pod repos" {
	run kwctl run  --request-path test_data/pod_creation.json  annotated-policy.wasm
	[ "$status" -eq 0 ]
	echo "$output"
	[ $(expr "$output" : '.*"allowed":true.*') -ne 0 ]
 }