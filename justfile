build:
    cargo build

test:
    cargo test

run:
    cargo run -- --http-only --http-port 8080

watch:
    cargo watch -s 'cargo run -- --http-only --http-port 8080'

deploy:
    ./deploy_on_push.sh
