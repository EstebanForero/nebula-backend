# Nebula Backend - Real-Time Chat Service

Nebula Backend implements the server-side of a real-time chat application with WebSockets, room management, message persistence, notifications, authentication, and basic observability. This README describes how to run, configure, and test the backend. It is not the full technical document.

## Features

* JWT authentication.
* Public and private chat rooms.
* Join and leave rooms.
* Real-time messaging via WebSockets.
* Message persistence in PostgreSQL.
* Paginated message history via REST.
* User-enter/exit notifications via RabbitMQ.
* Observability with Prometheus metrics and Grafana dashboards.
* Deployment through Docker Compose.

## Architecture Overview

Main backend components:

* Nebula Backend (Rust + Axum + Tokio): REST API, WebSocket server, metrics.
* PostgreSQL: persistent storage for users, rooms, messages, memberships.
* RabbitMQ: message broker for notifications.
* Redis (optional): cache or pub/sub.
* Notification Service: handles WebPush subscriptions and sends notifications.
* Prometheus and Grafana: metrics collection and visualization.

## Requirements

* Docker and Docker Compose.
* (Optional) Rust + Cargo for local development without Docker.

## Environment Variables

Backend (.env example):

```
BACKEND_ADDR=0.0.0.0:3838
DEV_MODE=true
DATABASE_URL=postgres://nebula:nebula123@postgres:5432/nebula
REDIS_URL=redis://redis:6379
RABBITMQ_HOST=rabbitmq
RABBITMQ_PORT=5672
RABBITMQ_USERNAME=nebula
RABBITMQ_PASSWORD=nebula123
RABBITMQ_VHOST=nebula
JWT_SECRET=zdwrJg3LT...
```

Notification service (.env example):

```
DATABASE_URL=postgres://nebula:nebula123@postgres:5432/nebula
JWT_SECRET=zdwrJg3LT...
RABBIT_URL=amqp://nebula:nebula123@rabbitmq:5672/nebula
RABBIT_QUEUE=room_member.notifications
RESEND_API_KEY=re_...
FROM_EMAIL=noreply@nebula.example.com
WEBPUSH_VAPID_PUBLIC_KEY=...
WEBPUSH_VAPID_PRIVATE_KEY=...
WEBPUSH_VAPID_SUBJECT=mailto:admin@nebula.example.com
HTTP_PORT=3010
DEV_MODE=true
```

## Running with Docker Compose

From the project root:

```
docker compose build
docker compose up -d
```

Services availability:

* Backend: [http://localhost:3838](http://localhost:3838)
* PostgreSQL: localhost:5432
* Redis: localhost:6379
* RabbitMQ UI: [http://localhost:15672](http://localhost:15672)
* Prometheus: [http://localhost:9090](http://localhost:9090)
* Grafana: [http://localhost:3050](http://localhost:3050)
* Notification service: [http://localhost:3010](http://localhost:3010)

To shut down:

```
docker compose down
```

## REST Endpoints Summary

Authentication:

* POST /auth/register
* POST /auth/login

User:

* GET /me

Rooms:

* GET /rooms/public
* GET /rooms
* POST /rooms

Room Membership:

* GET /rooms/{room_id}/members
* POST /rooms/{room_id}/members
* DELETE /rooms/{room_id}/members/me

Messages:

* GET /rooms/{room_id}/messages?page=&page_size=
* POST /rooms/{room_id}/messages

WebSocket:

* GET /ws/rooms/{room_id}?token=<JWT>

Health and Metrics:

* GET /
* GET /health
* GET /metrics

Notifications (notification service):

* POST /webpush/subscribe

## Testing

Unit and integration tests:

```
cargo test
```

If using docker-compose.test.yml:

```
docker compose -f docker-compose.test.yml up -d
cargo test
docker compose -f docker-compose.test.yml down
```

## Load Testing

Use your preferred test tool (k6, JMeter, simulate.py). Place scripts in a dedicated folder and document usage. Example:

```
python scripts/simulate.py --users 50 --rooms 5 --duration 120
```

## Observability

Prometheus scrapes backend metrics from /metrics. Grafana provides dashboards for latency, RPS, and error rates. Access Grafana at [http://localhost:3050](http://localhost:3050).

## Project Structure (example)

nebula-backend/
src/
migrations/
grafana/
.sqlx/
docker-compose.yml
docker-compose.test.yml
Dockerfile
Cargo.toml
README.txt
