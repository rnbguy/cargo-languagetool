# `cargo languagetool`

[![Crates.io Version][crates-io-badge]][crates-io-link]
[![MIT][license-image]][license-link]

> [!WARNING]\
> The command is still at alpha stage. Do not expect too much.

Improve the quality of your documentation. Use correct English words and
grammar, let anyone understand your documentation, allow anyone to use your
code, and reduce the time people need to spend to know how the crate works and
what it does. Good examples are necessary, but correct spelling and
understandable explanations are worth no less.

This is a fork of [`cargo-grammarly`][cargo-grammarly]. Thanks to [`iddm`][iddm]
for their prior work.

Grammarly [discontinued their developer API](grammarly-dev-api-discontinue). So,
I decided to switch to [LanguageTool][languagetool]. LanguageTool is free,
open-source and has a free public API.

# Installing

```sh
cargo install --git https://github.com/rnbguy/cargo-languagetool
```

# Using

```sh
cargo languagetool
# or
cargo languagetool src/
```

```console
$ cargo languagetool --help
A third-party cargo extension for checking grammar in the documentation and comments.

Usage: cargo languagetool [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  [default: .]

Options:
      --hostname <HOSTNAME>
          [env: LANGUAGETOOL_HOSTNAME=] [default: https://api.languagetoolplus.com]
  -p, --port <PORT>
          [env: LANGUAGETOOL_PORT=]
  -u, --username <USERNAME>
          [env: LANGUAGETOOL_USERNAME=]
  -a, --api-key <API_KEY>
          [env: LANGUAGETOOL_API_KEY=]
      --disable-categories <DISABLE_CATEGORIES>

      --enable-categories <ENABLE_CATEGORIES>

      --disable-rules <DISABLE_RULES>

      --enable-rules <ENABLE_RULES>

      --enable-only

      --language <LANGUAGE>
          [default: en-US]
      --picky

      --no-cache
          Disable cache query.
      --show-all
          Show all doc comments (even cached).
  -h, --help
          Print help
  -V, --version
          Print version
```

# Configuring

The utility works out of the box. However, if you want to use your
[premium key][languagetool-api-key], you may want to put it in the `.env` file
or as an environment variable as:

```
LANGUAGETOOL_USERNAME=jane@doe.com
LANGUAGETOOL_API_KEY=1234abcd
```

If you would like to use your own server, you can put the server URL in the
`.env` or as an environment variable as:

```
LANGUAGETOOL_HOSTNAME=http://localhost
LANGUAGETOOL_PORT=8010
```

> [!TIP]\
> You can use `ltrs docker` (`languagetool-rust` CLI) to launch a local
> `languagetool` docker container.

# How it works

The utility simply grabs all the doc comments (`///`, `//!`, `#![doc = "text"]`
and `#[doc = "text"]`) from your crate's source code and sends it to the
[`languagetool`][languagetool] grammar checking API using the
[`languagetool-rust`][languagetool-rust] crate. If there are any mistakes in
your texts, they are printed using the way the `rustc` compiler prints its
warnings and errors, using the [`annotate-snippets`][annotate-snippets] crate.

The doc comments are parsed using the `syn` and `proc_macro2` crates. These are
used specifically to know where in the code these comments are. Doing it with
regular expressions would waste a lot of time.

[license-image]: https://img.shields.io/badge/License-MIT-yellow
[license-link]: https://github.com/rnbguy/cargo-languagetool/blob/main/LICENSE
[crates-io-link]: https://crates.io/crates/cargo-languagetool
[crates-io-badge]: https://img.shields.io/crates/v/cargo-languagetool
[languagetool]: https://languagetoolplus.com
[languagetool-api-key]: https://languagetool.org/editor/settings/access-tokens
[languagetool-rust]: https://crates.io/crates/languagetool-rust
[annotate-snippets]: https://crates.io/crates/annotate-snippets
[iddm]: https://github.com/iddm
[cargo-grammarly]: https://github.com/iddm/cargo-grammarly
[grammarly-dev-api-discontinue]: https://developer.grammarly.com
