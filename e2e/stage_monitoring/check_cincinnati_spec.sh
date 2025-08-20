#!/bin/bash

# Call Slack workflow stage_cincinnati to send messages
function send_slack() {
	local message="$1"
	# slack-url is coming from secret slack-workflow
	curl -X POST "${SLACK_URL}" -d '{"msg": "'"${message}"'"}'
}

# check paths/graph/parameters
# parameter:
# @parameter: arch, channel, version, id
# @path: the parameter path
# @res: the origin response
function check_parameter() {
    local parameter="$1" path="$2" res="$3" 

    DATE="$(date --iso=s --utc)"; 
    param=$(echo "${res}" | jq "${path}.parameters")
    value=$(echo "$param" | jq -r '.[] | select(.name=="'"${parameter}"'")')
    if [[ -z "${value}" ]] ; then
        echo -e "${DATE} Error: failed to get ${parameter} from openapi \n ${param}"
        return 1
    fi

    echo "The parameter's spec from graph API is correct: ${parameter}"

    return 0
}

# check .components.schemas.Graph.properties
# parameter:
# @property: nodes, edges, conditionalEdges
# @expected_type: the expected value of .components.schemas.Graph.properties.@property.type
# @expected_items: the expected value of .components.schemas.Graph.properties.@property.items
# @res: the origin response
function check_property() {
        local property="$1" expected_type="$2" expected_items="$3" res="$4" 

        result=$(echo "${res}" | jq ".components.schemas.Graph.properties.${property}")
        type=$(echo "$result" | jq -r ".type")
        if [[ "${type}" != "${expected_type}" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Graph/properties/${property}/type from openapi"
            return 1
        fi
        ref=$(echo "$result" | jq -r '.items."$ref"')
        if [[ "${ref}" != "${expected_items}" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Graph/properties/${property}/items from openapi"
            return 1
        fi
        echo "Spec components.schemas.Graph.properties.${property} from graph API is correct"
        return 0
}

# check response in different HTTP status
# parameter:
# @http_code: 200, 400, 406, 500, default
# @path: '.paths."/api/upgrades_info/graph"' or '.paths."/api/upgrades_info/v1/graph"'
# @res: the origin response
function check_response() {
    local http_code="$1" path="$2" res="$3" 

    DATE="$(date --iso=s --utc)"; 
    responses=$(echo "${res}" | jq "${path}.get.responses")
    content=$(echo "${responses}" | jq '."'"${http_code}"'"' | jq ".content")
    responses_ref=$(echo "$content" | jq -r '."application/json".schema."$ref"')

    if [[ "${http_code}" == "200" ]]; then
        if [[ "${responses_ref}" != "#/components/schemas/Graph" ]] ; then
            echo -e "${DATE} Error: failed to get responses/200/content/application/json from openapi: ref \n ${responses}"
            return 1
        fi
        responses_new_ref=$(echo "$content" | jq -r '."application/vnd.redhat.cincinnati.v1+json".schema."$ref"')
        if [[ "${responses_new_ref}" != "#/components/schemas/Graph" ]] ; then
            echo -e "${DATE} Error: failed to get responses/200/content/application/vnd.redhat.cincinnati.v1+json from openapi: ref \n ${responses}"
            return 1
        fi
    else
        if [[ "${responses_ref}" != "#/components/schemas/GraphError" ]] ; then
            echo -e "${DATE} Error: failed to get responses/${http_code}/content/application/json from openapi: ref \n ${responses}"
            return 1
        fi
    fi

    echo "The responce spec from graph API is correct: ${http_code}"
    return 0
}

openapi="https://api.stage.openshift.com/api/upgrades_info/openapi"
openapi_retry=0
while sleep 10m; 
do 
	echo -e "\n\n"
	echo "=================check openapi================="
	if [[ $openapi_retry -lt 2 ]]; then
        res=$(curl -s "${openapi}")

        DATE="$(date --iso=s --utc)"; 
        openapi_version=$(echo "${res}" | jq ".components.schemas.Version")
        openapi_version_type=$(echo "$openapi_version" | jq -r ".type")
        if [[ "${openapi_version_type}" != "string" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Version from openapi: type"
            ((openapi_retry += 1))
            continue
        fi
        openapi_version_description=$(echo "$openapi_version" | jq -r ".description")
        if [[ "${openapi_version_description}" != "The version of an OpenShift release" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Version from openapi: description"
            ((openapi_retry += 1))
            continue
        fi
        openapi_version_example=$(echo "$openapi_version" | jq -r ".example")
        if [[ -z "${openapi_version_example}" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Version from openapi: example"
            ((openapi_retry += 1))
            continue
        fi
        echo "components.schemas.Version spec from graph API is correct"

        openapi_properties_version=$(echo "${res}" | jq ".components.schemas.Graph.properties.version")
        openapi_properties_version_type=$(echo "$openapi_properties_version" | jq -r ".type")
        if [[ "${openapi_properties_version_type}" != "integer" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Graph/properties/version from openapi: type"
            ((openapi_retry += 1))
            continue
        fi
        openapi_properties_version_example=$(echo "$openapi_properties_version" | jq -r ".example")
        if [[ "${openapi_properties_version_example}" != "1" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Graph/properties/version from openapi: example"
            ((openapi_retry += 1))
            continue
        fi
        openapi_properties_version_min=$(echo "$openapi_properties_version" | jq -r ".minimum")
        if [[ "${openapi_properties_version_min}" != "1" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Graph/properties/version from openapi: minimum"
            ((openapi_retry += 1))
            continue
        fi
        openapi_properties_version_max=$(echo "$openapi_properties_version" | jq -r ".maximum")
        if [[ "${openapi_properties_version_max}" != "2147483647" ]] ; then
            echo "${DATE}" "Error: failed to get schemas' Graph/properties/version from openapi: maximum"
            ((openapi_retry += 1))
            continue
        fi
        echo "Spec components.schemas.Graph.properties.version from graph API is correct"

        if ! check_property "nodes" "array" "#/components/schemas/Node" "${res}"; then
            ((openapi_retry += 1))
            continue
        fi
        if ! check_property "edges" "array" "#/components/schemas/Edge" "${res}"; then
            ((openapi_retry += 1))
            continue
        fi
        if ! check_property "conditionalEdges" "array" "#/components/schemas/ConditionalEdges" "${res}"; then
            ((openapi_retry += 1))
            continue
        fi


		for path in '.paths."/api/upgrades_info/graph"' '.paths."/api/upgrades_info/v1/graph"'
		do
			echo "------------------Check path ${path}------------------"
            if ! check_parameter "id" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
            if ! check_parameter "channel" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
            if ! check_parameter "arch" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
            if ! check_parameter "version" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi

            if ! check_response "200" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
            if ! check_response "400" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
            if ! check_response "406" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
            if ! check_response "500" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
            if ! check_response "default" "${path}" "${res}"; then
                ((openapi_retry += 1))
                continue
            fi
		done
	fi
	if [[ $openapi_retry -ge 2 ]]; then
        DATE="$(date --iso=s --utc)"; 
        echo "${DATE} Error: failed to check response schema of openapi"
		send_slack "${DATE} Error: failed to check response schema of openapi"
        # send message once
        ((openapi_retry += 1))
	fi
done