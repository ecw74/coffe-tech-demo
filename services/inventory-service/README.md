## 📦 Inventory Service – Detailed Description

### 🧩 Purpose

The **Inventory Service** is responsible for managing the available stock of ingredients – specifically coffee beans and
milk. It provides a REST API for querying the current inventory and for refilling ingredients. The Machine Service uses
this API to request and consume ingredients.

---

### 🌐 REST API Endpoints

#### `GET /fill`

- **Description:** Returns the current inventory levels for beans and milk.
- **Response (JSON):**
  ```json
  {
    "beans": 7,
    "milk": 3
  }
  ```

#### `PUT /fill`

- **Description:** Adds new ingredients to the inventory. Only the provided fields are updated.
- **Request Body (JSON):**
  ```json
  {
    "beans": 10,
    "milk": 5
  }
  ```
- **Response (JSON):**
  ```json
  {
    "message": "Inventory updated",
    "beans": 17,
    "milk": 8
  }
  ```

---

### ⚙️ Internal Logic

- Inventory is stored either in-memory or using lightweight persistence (e.g. SQLite or Redis).
- Partial updates are supported: e.g. only `milk` can be increased.
- Values are always **added** to the current stock.
- No negative values allowed; validation is enforced.

---

### 🧠 Usage by Machine Service

The Machine Service calls `GET /fill` to check current stock and then `PUT /fill` to deduct ingredients after preparing
a drink.

| Drink      | Beans | Milk |
|------------|-------|------|
| Espresso   | 1     | 0    |
| Coffee     | 2     | 0    |
| Cappuccino | 1     | 1    |

---

### 🛠 Example Tech Stack

- **Language:** Rust
- **Web Framework:** Axum
- **Storage:** InMemory
- **Containerization:** Docker-ready

---

### 🔐 Validation & Error Handling

- Input validation for non-negative integers
- Missing fields are ignored during `PUT /fill`
- Proper HTTP status codes used (`400` for bad requests, `200` for success)

---

### 🧪 Example Requests

**Check current inventory:**

```http
GET /fill
```

→ Response:

```json
{
  "beans": 7,
  "milk": 3
}
```

**Refill inventory:**

```http
PUT /fill
Content-Type: application/json

{
  "beans": 5
}
```

→ Response:

```json
{
  "message": "Inventory updated",
  "beans": 12,
  "milk": 3
}
```

---

### 😄 Optional Fun Feature

When inventory drops below a critical level (e.g., less than 2 beans), the service could log a warning:

```
WARNING: Bean levels critically low – consider caffeine emergency protocol ☕🚨
```
