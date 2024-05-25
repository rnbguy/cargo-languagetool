alias fmt := format

format:
    taplo fmt
    cargo +nightly fmt
    just --fmt --unstable
