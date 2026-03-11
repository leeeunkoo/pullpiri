#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

BODY=$(< ./resources/helloworld_no_condition.yaml)
# BODY=$(< ./resources/vss_scenario.yaml)

#URL="10.0.0.30:8080/api/v1/yaml"

curl -X POST 'http://192.168.10.2:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"
