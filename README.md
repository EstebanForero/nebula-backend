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

<img width="871" height="534" alt="image" src="https://github.com/user-attachments/assets/39c5f4f1-ade1-4fd9-a33d-f73c4651f553" />


### Message Delivery Flow

1. A client sends a message via `POST /rooms/{id}/messages`.
2. The Rust backend persists the message to PostgreSQL.
3. The backend publishes the message to Redis Pub/Sub on a room-specific channel.
4. All WebSocket worker instances subscribed to that channel evaluate:

   * If the connected client is in the same room
   * If the connected client is not the sender
5. Valid WebSocket clients receive the message in real time.
6. A notification event is published to RabbitMQ so the Bun notification service can handle push notifications asynchronously.

<img width="1395" height="893" alt="flow-diagram" src="https://github.com/user-attachments/assets/49f585b1-ae65-4062-b49d-f5132832f8ef" />

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

## Rust Backend Tests

### Unit tests

Inside `nebula-backend/`:

```
export SQLX_OFFLINE=true 
cargo test
```

<img width="734" height="893" alt="image" src="https://github.com/user-attachments/assets/eda89167-743f-4472-98df-45b83560578b" />


This runs unit tests.

### Integration Tests (Docker)

Inside `nebula-backend/`:

```
./test/run_integration.sh
```

This spins up the ephemeral test stack defined in `docker-compose.test.yml` (Postgres on 55432, Redis on 36379, RabbitMQ on 5674/15674), runs the auth/room integration suites serially, and tears everything down. Use `TEST_DATABASE_URL`, `TEST_REDIS_URL`, and `TEST_JWT_SECRET` to override defaults if needed.

<img width="478" height="170" alt="image" src="https://github.com/user-attachments/assets/88940948-8daf-44d6-ae62-f15d95bef756" />

<img width="575" height="311" alt="image" src="https://github.com/user-attachments/assets/76645539-e4ad-4847-a8c8-66ebcb118c85" />


## Notification Service Tests

### Unit tests

Inside `nebula-notification-service/`:

```
bun test
```

<img width="812" height="557" alt="image" src="https://github.com/user-attachments/assets/5c51074a-f14c-437a-9f25-2febafdecbf2" />


Tests include subscription parsing, push registration validation, and AMQP consumer behavior (mocked).

### Integration tests

Integration (starts Postgres/RabbitMQ via Compose, runs tests locally, then tears down): 

```
bun run test:integration:docker
```

<img width="722" height="522" alt="image" src="https://github.com/user-attachments/assets/def193cf-c08b-4861-9406-e6f16f370699" />


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

<img width="1733" height="1780" alt="image" src="https://github.com/user-attachments/assets/01b0a7f7-7bc6-4cf5-835c-8ed58bc21762" />

