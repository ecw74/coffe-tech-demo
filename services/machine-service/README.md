## 🤖 Machine Service – Detailed Description

### 🧩 Purpose

The **Machine Service** is responsible for processing coffee orders. It consumes messages from the RabbitMQ queue
`order.placed`, retrieves the necessary ingredients from the Inventory Service via REST, and simulates beverage
preparation. It also exposes a REST endpoint to provide its current operational status.

---

### 📬 Messaging (RabbitMQ)

- **Queue:** `order.placed`
- **Message format:**
  ```text
  {
    "order_id": "abc-123",
    "type": "espresso" | "kaffee" | "cappuccino",
    "timestamp": "2025-06-11T18:42:00Z"
  }
  ```

- **Processing logic:**
    1. Receive message from `order.placed`
    2. Determine ingredient requirements based on drink type
    3. Check current stock via `GET /fill` from Inventory Service
    4. Deduct ingredients using `PUT /fill` request
    5. Simulate preparation (e.g., sleep or log)
    6. Update internal status

---

### ☕ Ingredient Requirements

| Drink      | Beans | Milk |
|------------|-------|------|
| Espresso   | 1     | 0    |
| Coffee     | 2     | 1    |
| Cappuccino | 1     | 2    |

---

### 🌐 REST API Endpoints

#### `GET /status`

- **Description:** Returns the current status of the machine.
- **Response (JSON):**
  ```json
  {
    "ready": true,
    "last_order": {
      "order_id": "abc-123",
      "type": "espresso",
      "status": "done",
      "finished_at": "2025-06-11T18:45:00Z"
    }
  }
  ```

---

### ⚠️ Error Handling

- If ingredients are insufficient:
    - Order is skipped or logged as failed
    - Optionally send to `order.failed` queue
    - Example log:
      ```
      ERROR: Not enough milk for cappuccino (order_id: abc-123)
      ```

---

### 🛠 Example Tech Stack

- **Language:** Rust
- **Messaging:** RabbitMQ Consumer `lapin`
- **Web Framework:** Axum
- **HTTP Client:** reqwest
- **Containerization:** Docker-ready

---

### 🧪 Example Response

```http
GET /status
```

→ Response:

```json
{
  "ready": true,
  "last_order": {
    "order_id": "def-456",
    "type": "kaffee",
    "status": "done",
    "finished_at": "2025-06-11T18:46:30Z"
  }
}
```

---

### 😄 Optional Fun Feature

If three cappuccinos are processed in a row, the machine might log:

```
Machine thinks you're getting fancy ☕🎩
```
