# tests/

Integration test infrastructure — not application unit tests (those live inside each crate's own
`tests/` directory or in `core/tests/conformance/`).

## Contents

```
tests/
└── caldav-servers/    Docker Compose stacks for CalDAV server integration tests
    └── radicale/      Radicale config — lightweight CalDAV server used in CI
```

## Running CalDAV integration tests

```bash
# 1. Start the CalDAV server
cd tests/caldav-servers && docker compose up -d

# 2. Run the integration tests (marked ignored by default, need --include-ignored)
cd core && cargo test -p songbird-caldav-client -- --include-ignored

# 3. Tear down
cd tests/caldav-servers && docker compose down
```
