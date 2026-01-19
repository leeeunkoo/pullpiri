/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
#![allow(non_snake_case)]

use common::spec::artifact::Scenario;

const S: &str = r#"apiVersion: v1
kind: Scenario
metadata:
  name: bms
spec:
  condition:
    express: eq
    value: D
    operands:
      type: DDS
      name: gear
      value: PowertrainTransmissionCurrentGear
  action: update
  target: bms-algorithm-performance
status:
  state: Waiting"#;

fn main() {
    println!("{}\n", S);

    let result = serde_yaml::from_str::<Scenario>(S).unwrap();
    println!("{:#?}\n", result);

    let recover = serde_yaml::to_string(&result).unwrap();
    println!("{:#?}\n", recover);
}
