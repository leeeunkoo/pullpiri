#!/bin/bash

#BODY=$(< ./resources/helloworld.yaml)
BODY=$(< ./resources/helloworld_no_condition.yaml)

#URL="10.0.0.30:8080/api/v1/yaml"

curl -X DELETE 'http://10.0.0.30:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"
