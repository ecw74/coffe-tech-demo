# ☕ Coffee Microservice Architecture

This project demonstrates a fun, modular microservice architecture for a fictional coffee ordering system. It is
designed with clear separation of responsibilities, asynchronous communication, and pragmatic simplicity.

Each service is implemented in Rust using [Axum](https://docs.rs/axum/latest/axum/) and communicates via REST and
RabbitMQ.

---

## 🧱 Architecture Overview

```
               +-------------+
               | Order       |
               |  Service    |
               +-------------+
                     |
         POST /order | GET /orders/queue-length
                     v
               +-------------+
               | RabbitMQ    |
               | order.placed|
               +-------------+
                     |
                     v
               +-------------+         REST        +----------------+
               | Machine     | <-----------------> | Inventory      |
               |  Service    |  GET/PUT/DEL /fill  |   Service      |
               +-------------+                     +----------------+
                     |
           GET /status (machine state)
```

---

## 🧩 Service Descriptions

### ☕ Order Service

- Accepts orders for `espresso`, `coffee`, and `cappuccino` via `POST /order`
- Publishes orders to RabbitMQ queue `order.placed`
- Exposes `GET /orders/queue-length` to monitor queue size

📖 [More details → Order Service README](./services/order-service/README.md)

---

### 📦 Inventory Service

- Manages current stock of beans and milk
- Exposes `GET /fill`, `PUT /fill` and `DEL /fill` to query, refill and remove ingredients
- Used exclusively by the Machine Service to request or deduct inventory

📖 [More details → Inventory Service README](./services/inventory-service/README.md)

---

### 🤖 Machine Service

- Consumes messages from `order.placed` queue
- Checks and deducts ingredients via the Inventory Service
- Simulates drink preparation and maintains status via `GET /status`

📖 [More details → Machine Service README](./services/machine-service/README.md)

---

## 🧪 API Documentation (Swagger UI)

Each microservice includes an interactive Swagger UI to explore and test its API directly in the browser.

| Service           | Swagger UI URL                   |
|-------------------|----------------------------------|
| Order Service     | http://localhost:8080/swagger-ui |
| Inventory Service | http://localhost:8081/swagger-ui |
| Machine Service   | http://localhost:8082/swagger-ui |

---

## 🐋 Running the System with Docker Compose

### 🔧 Requirements

- Docker
- Docker Compose v2+

### ▶️ Start the full system

```bash
docker-compose up --build
```

This command will:

- Build the three services (order, inventory, machine)
- Start a RabbitMQ container with the Management UI at http://localhost:15672 (default login: `user` / `pass`)
- Expose the services on ports:

    - Order: `localhost:8080`
    - Inventory: `localhost:8081`
    - Machine: `localhost:8082`

Swagger UI is available at:

- `http://localhost:8080/swagger-ui`
- `http://localhost:8081/swagger-ui`
- `http://localhost:8082/swagger-ui`

---

## 🧪 Try It Out

You can interact with the system using:

- [HTTPie](https://httpie.io/cli)
- `curl`
- A REST client like Postman or JetBrains HTTP Client

Example: Place an order

```bash
curl -X POST http://localhost:8080/order -H "Content-Type: application/json" -d '{ "type": "espresso" }'
```

---

## 🔐 Notes

- The system is stateless by default; ingredient levels reset on restart unless persistence is added.
- RabbitMQ queue must be available before Machine Service starts consuming. Docker Compose handles this via dependency.

---

## 😄 Extras

- If you try ordering `"tea"` you’ll get a friendly reminder that this is a coffee-only establishment.
- The Machine Service logs messages if too many cappuccinos are ordered — because it's watching you get fancy. ☕🎩

---

Enjoy your coffee-powered microservices! Questions welcome. ☕🚀
