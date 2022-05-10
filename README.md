# ALUP HA 5 - ELF parsing
This program parses ELF files in given folder, finds required dynamic libraries, and lists them with respective dependant executables.

## Requirements
rust 1.58.1+

## Usage
Run the program with `cargo run` specifying folder with binaries as an cmd argument:

`cargo run -- -e /usr/bin`

By default `/` folder is considered.