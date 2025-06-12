## ☕ Order Service – Detailed Description

### 🧩 Purpose
The **Order Service** acts as the system’s entry point. It exposes a REST API that allows users to place orders for `espresso`, `coffee`, or `cappuccino`. Each order is published as a message to the RabbitMQ queue `order.placed`. The service is not responsible for processing the order itself.

---

### 🌐 REST API Endpoints

#### `POST /order`

- **Description:** Submit a new coffee order
- **Request Body (JSON):**
  ```text
  {
    "type": "espresso" | "coffee" | "cappuccino"
  }
  ```
- **Responses:**
    - `202 Accepted` – Order was accepted and queued
    - `400 Bad Request` – Invalid drink type

#### `GET /orders/queue-length`

- **Description:** Returns the number of unprocessed messages in the `order.placed` queue
- **Response (JSON):**
  ```json
  {
    "pending_coffee_orders": 2
  }
  ```

---

### 📬 Messaging (RabbitMQ)

- **Queue:** `order.placed`
- **Published Message Format:**
  ```json
  {
    "order_id": "abc-123",
    "type": "espresso",
    "timestamp": "2025-06-11T18:42:00Z"
  }
  ```

---

### 🛠 Example Tech Stack

- **Language:** Rust
- **Web Framework:** Axum
- **Messaging:** RabbitMQ Producer `rabbitmq-stream-client`
- **Containerization:** Docker-ready

---

### 🔐 Validation & Resilience

- Validates order types (must be one of the supported drinks)
- Includes basic RabbitMQ reconnection logic
- Queue length is retrieved using either the RabbitMQ Management API or a passive queue inspection method

---

### 🧪 Example Requests

**Order a drink:**

```http
POST /order
Content-Type: application/json

{
  "type": "cappuccino"
}
```

→ Response:
```json
{
  "message": "Order received",
  "order_id": "abc-123"
}
```

**Check queue length:**

```http
GET /orders/queue-length
```

→ Response:
```json
{
  "pending_coffee_orders": 2
}
```

---

### 😄 Optional Fun Feature

If someone tries to order `"tea"`, the response could be:

```json
{
  "error": "This is a coffee-only establishment ☕"
}
```
