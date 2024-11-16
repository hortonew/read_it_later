# Read it Later

An implementation of a read it later or bookmark manager style app (e.g. Pocket, Omnivore, Raindrop.io, etc.).

API: Rust and Actix Web
Database: Postgres
Cache: Redis (not implemented yet)
Client: Chrome Extension

## Configure

Create .env, updating with your desired users and passwords.

```ini
POSTGRES_USER=user
POSTGRES_PASSWORD=password
POSTGRES_DB=read_later
REDIS_PASSWORD=password
DATABASE_URL=postgres://user:password@db:5432/read_later
REDIS_URL=redis://:password@redis:6379
INDEX_RESPONSE="Welcome to the Read it Later app!"
WEB_PORT=8080
PACKAGE_NAME=read_it_later
POSTGRES_PORT=5432
REDIS_PORT=6379
```

## Run

```sh
docker compose up
# navigate to http://localhost:8080/saves
```

## Chrome Extension

1. Go to chrome://extensions/
2. Enable developer mode
3. Click "Load Unpacked"
4. Open the directory "chrome-extension"
5. Pin the URL Poster app, navigate to a url, and submit a url
