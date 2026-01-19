/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
mod cli;
mod commands;

fn main() {
    let args = cli::parse();

    if args.logo {
        commands::print_logo();
    }

    match args.command {
        cli::Commands::Status => commands::status::run(),
        cli::Commands::Apply(i) => commands::apply::run(i),
        cli::Commands::Delete(i) => commands::delete::run(i),
    }
}
