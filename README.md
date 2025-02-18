# cargo-check-delta

## What is this?

This cargo subcommand was created to solve the problem of rust-analyzer not
working at all in a mixed workspace of crates with many different targets and
features.

In addition to the diagnostics that rust-analyzer does on its own, it uses
`cargo check` at save time in the root of the workspace to get accurate
diagnostics. However, in a project like an embedded one, which contains many
dependencies and such that require specific feature flags to build, this will
only generate a lot of useless error messages. One solution to this is to run
diagnostics on a per-directory basis. This way, each crate's FEATURE will be
properly resolved and the appropriate diagnostics will be generated. However,
such a configuration does not currently exist in rust-analyzer, so this
subcommand is useful.

And this subcommand does not just run `cargo check` on all crates. It saves a
file in the target folder with the source code path and its modification date
and time, and then detects the modified file the next time it is run, thereby
running `cargo check` only on the modified crate. This allows the

## Usage

This is used through `rust-analyzer.check.command` or
`rust-analyzer.check.overrideCommand` config of rust-analyzer.

For example, you can write below config in `.vscode/settings.json`.

```json
{
  "rust-analyzer.check.overrideCommand": [
    "cargo",
    "check-delta",
    "--message-format=json"
  ]
}
```
