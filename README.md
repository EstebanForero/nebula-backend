# Nebula Real-Time Chat System

Final Project – Patrones Arquitectónicos Avanzados
Universidad de La Sabana – 2025

Nebula is a real-time chat application designed to demonstrate architectural patterns such as client–server communication, WebSockets, event-driven messaging, horizontal scalability, observability, and asynchronous processing.
The system fulfills all the functional and non-functional requirements described in the course assignment.

---

# Overview

Nebula consists of a web client, a primary backend service implemented in Rust, and a secondary backend service implemented in Bun/TypeScript responsible for processing notifications. The system integrates multiple infrastructure components including Redis, RabbitMQ, and PostgreSQL.

The project supports:

* Public and private chat rooms
* JWT-based authentication
* Paginated message history via REST
* Real-time message delivery via WebSockets
* Asynchronous notification processing
* Horizontal scalability for WebSocket fan-out
* Persistence of all confirmed messages in PostgreSQL
* Basic observability: latency metrics, connection metrics, and structured logs
* Load testing with k6

---

# Architecture Summary

Nebula uses the minimum architecture required:

### Components

| Component              | Technology               | Description                                                                                        |
| ---------------------- | ------------------------ | -------------------------------------------------------------------------------------------------- |
| Web Client             | React + TypeScript       | Provides UI for authentication, chat, rooms, and notifications.                                    |
| API + WebSocket Server | Rust (Axum, SQLx, Tokio) | Handles REST endpoints, JWT auth, message persistence, room management, and WebSocket connections. |
| Notification Service   | Bun + TypeScript         | Consumes RabbitMQ events to process and deliver push notifications.                                |
| Redis                  | Pub/Sub                  | Used for message fan-out across WebSocket worker instances and room message replication.           |
| RabbitMQ               | AMQP queue               | Used for asynchronous notification workloads and smoothing traffic spikes.                         |
| PostgreSQL             | Relational DB            | Stores users, rooms, messages, memberships, and metadata.                                          |

### Message Delivery Flow

1. A client sends a message via `POST /rooms/{id}/messages`.
2. The Rust backend persists the message to PostgreSQL.
3. The backend publishes the message to Redis Pub/Sub on a room-specific channel.
4. All WebSocket worker instances subscribed to that channel evaluate:

   * If the connected client is in the same room
   * If the connected client is not the sender
5. Valid WebSocket clients receive the message in real time.
6. A notification event is published to RabbitMQ so the Bun notification service can handle push notifications asynchronously.

### Access Control

* JWT required for authenticated operations.
* Public rooms require no password.
* Private rooms require password verification.
* Room creators are automatically joined as members.
* WebSocket connections require a token in the query string.

---

# Repository Structure

```
nebula-frontend/              React client
nebula-backend/               Rust backend (Axum + SQLx)
nebula-notification-service/  Bun/TypeScript service (RabbitMQ consumer)
docker-compose.yml            Local development stack
k6/nebula_latency.test.js     Load and latency simulation script
```

---

# Running Locally (Docker Compose)

The full development stack can be started using:

```
docker compose up --build
```

This brings up:

* PostgreSQL
* Redis
* RabbitMQ
* Nebula REST + WebSocket backend (Rust)
* Notification service (Bun)
* Optional frontend service (React)

Default service endpoints:

* API: [http://localhost/api/backend(http://localhost/api/backend)
* Notifications: [http://localhost/api/notifications](http://localhost/api/notifications)
* Frontend: [http://localhost(http://localhost)

---

# Running Tests

### Rust Backend Tests

Inside `nebula-backend/`:

```
cargo test
```

This runs unit tests and integration tests, including SQLx query tests and WebSocket logic tests.

### Integration Tests (Docker)

Inside `nebula-backend/`:

```
./test/run_integration.sh
```

This spins up the ephemeral test stack defined in `docker-compose.test.yml` (Postgres on 55432, Redis on 36379, RabbitMQ on 5674/15674), runs the auth/room integration suites serially, and tears everything down. Use `TEST_DATABASE_URL`, `TEST_REDIS_URL`, and `TEST_JWT_SECRET` to override defaults if needed.

### Notification Service Tests

Inside `nebula-notification-service/`:

```
bun test
```

Tests include subscription parsing, push registration validation, and AMQP consumer behavior (mocked).

---

# Load Testing & Latency Validation (k6)

The project includes a full k6 simulation script:

```
k6/nebula_latency.test.js
```

It performs:

* Automatic creation of temporary test users
* One dedicated REST sender
* One WebSocket receiver
* One hundred concurrent WebSocket receivers (stress scenario)
* Measurement of:

  * HTTP latency metrics
  * WebSocket connection times
  * End-to-end message delivery latency (HTTP POST → WS receive)

### Running the Load Test

```
k6 run k6/nebula_latency.test.js
```

An HTML report is generated after completion:

```
nebula-latency-report.html
```

---

# Performance Results

The non-functional requirement specifies:

* **Support for “dozens” of simultaneous users**
* **Message delivery latency below 850 ms**

The system was tested with **100 concurrent WebSocket receivers**, exceeding the "dozens" requirement.

### Real-Time Message Delivery Latency

Measured using the end-to-end metric (`POST` → `WebSocket receive`):

| Metric          | Result                  |
| --------------- | ----------------------- |
| Average latency | **24.58 ms**            |
| p90             | **71.00 ms**            |
| p95             | **239.00 ms**           |
| Max             | 240 ms (single outlier) |

All latency results are **well below the 850 ms requirement**, even at 100 concurrent WebSocket clients.

### HTTP Latencies

Key endpoints also satisfied performance expectations:

| Endpoint                    | Average                                                          | p90     | p95     |
| --------------------------- | ---------------------------------------------------------------- | ------- | ------- |
| `/me`                       | 0.60 ms                                                          | 0.76 ms | 0.94 ms |
| `/rooms`                    | 0.58 ms                                                          | 0.72 ms | 0.90 ms |
| `GET /rooms/{id}/messages`  | 0.67 ms                                                          | 1.07 ms | 1.11 ms |
| `POST /rooms/{id}/messages` | 5.29 ms median, variable under load but within acceptable ranges |         |         |

### Room Join

Room join requires validation, password handling, membership checks, and redis operations, so it is expectedly heavier:

* Average join time: **727.68 ms**

Still within acceptable limits and only done once per user.

---

# Metrics Summary

Insert metrics image(s) here:

```
[metric image here]
```
