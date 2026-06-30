# Self-hosting Songbird's sync service

See system-design.md §12.2. `docker-compose.yml` brings up the full stack (api-gateway,
Postgres event store, push relay, Redis, Caddy reverse proxy) once M4 lands.

Requirements (fill in before first real deployment):
- A domain + TLS (Caddy handles automatic HTTPS by default — point `Caddyfile` at your domain)
- UnifiedPush distributor config for self-hosters who don't want to depend on FCM/APNs (FCM/APNs
  remain the default-easy-path fallback)
- This deployment yields full feature parity with any managed tier, including E2EE — see
  system-design.md §8 for why that's a structural guarantee, not a configuration option.
