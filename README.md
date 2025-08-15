# Links Shortener

A full-stack URL shortening service with analytics, built with Rust (backend), React (frontend), and MongoDB. Production-ready with Docker Compose, robust UI/UX, and E2E tests.

## Features

- Shorten long URLs with a single click
- Real-time analytics for each short link
- Responsive, modern UI with dark mode
- Production-ready Docker setup
- MongoDB data persistence and backup
- E2E, integration, and unit tests

## Quick Start (Docker Compose)

```sh
git clone <repo-url>
cd links-shortener
cp backend/.env.example backend/.env
cp frontend/.env.example frontend/.env
# (Edit .env files as needed)
docker-compose up --build
```

- Frontend: <http://localhost:3000>
- Backend API: <http://localhost:8080>
- MongoDB: localhost:27017

## Local Development

- Backend: `cd backend && cargo run`
- Frontend: `cd frontend && npm install && npm run dev`
- MongoDB: Use Docker or local install

## Environment Variables

- See `backend/.env.example` and `frontend/.env.example` for all options
- For production, use `.env.production` files and secrets

## Testing

### Backend (Rust)

```sh
cd backend
cargo test
```

### Frontend (React)

```sh
cd frontend
npm run lint
# (Add unit tests as needed)
```

## Deployment & Scaling

- See [backend/README.md](./backend/README.md) for MongoDB backup/restore

## Troubleshooting

- Check logs: `docker-compose logs <service>`
- Ensure all environment variables are set
- For CORS/API issues, check backend CORS and Nginx proxy
- For MongoDB issues, check volume mounts and backup docs
- For E2E test failures, ensure frontend is running and accessible
- Check links in db:

```sh
docker exec -it links-shortener-mongo-1 mongosh shortener --eval 'db.urls.find().pretty()'
````

## Maintainer

- Alexander Extim (<it@extim.su>)
- [GitHub Issues](https://github.com/extimsu/links-shortener/issues)
