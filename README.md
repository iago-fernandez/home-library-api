# Home Library API

A high-performance, asynchronous REST API engineered for physical asset management. Built with **Rust**, **Axum**, and **PostgreSQL**, this backend features an advanced dynamic query engine and an asynchronous external metadata proxy. It is specifically designed to be deployed on a dedicated **[homelab](https://github.com/iago-fernandez/homelab)** environment, providing a centralized, highly efficient, and private inventory system.

[![Status](https://img.shields.io/badge/status-stable-green?style=flat-square)](https://github.com/iago-fernandez/home-library-api/releases)
[![Language](https://img.shields.io/badge/language-Rust%20-dea584?style=flat-square)](https://www.rust-lang.org/)
[![Framework](https://img.shields.io/badge/framework-Axum-8a2be2?style=flat-square)](https://github.com/tokio-rs/axum)
[![Database](https://img.shields.io/badge/database-PostgreSQL-336791?style=flat-square)](https://www.postgresql.org/)
[![Deployment](https://img.shields.io/badge/deployment-Docker%20-2496ed?style=flat-square)](Dockerfile)
[![License](https://img.shields.io/badge/license-MIT-orange?style=flat-square)](LICENSE)

## Core Engineering

Designed with a strict focus on memory safety, temporal efficiency, and architectural cleanliness:

* **Asynchronous I/O:** Built on top of the Tokio runtime and Axum web framework to handle highly concurrent network requests with minimal thread-blocking overhead.
* **Compile-Time SQL Verification:** Utilizes SQLx to ensure all database queries and schema architectures are validated against the PostgreSQL database during the compilation phase, eliminating runtime syntax errors.
* **Dynamic Query Engine:** Implements a custom parameter parsing algorithm that maps URL operational suffixes (e.g., `_gte`, `_contains`) directly to optimized SQL operators, allowing for infinite combinations of filters in a single request.
* **External Metadata Proxy:** Integrates `reqwest` to asynchronously resolve exact ISBNs and search terms against the Open Library API, streamlining data entry via a unified backend proxy.
* **Clean Code Philosophy:** Relies on strict typing, idiomatic Rust paradigms (let-chaining), and expressive naming conventions in place of redundant comments.

## Deployment Topology

This system is engineered to run as an isolated, containerized service.

* **Multi-Stage Containerization:** The `Dockerfile` implements a multi-stage build process. The heavy Rust compiler and source code exist only in the builder stage. Only the compiled, highly optimized binary (~80MB) is passed to the final Debian Slim runtime image, keeping the host system clean of development dependencies.
* **Homelab Integration:** Orchestrated via `docker-compose`, the API and the PostgreSQL database run on an internal Docker network with persistent volumes. The API port (`3000`) is exposed locally and accessed remotely via a secure Wireguard VPN connection, leaving port `443` completely free for public-facing services.

## Project Structure

The repository maintains a strict separation of concerns between network routing, data transfer objects, external integrations, and database interactions:

```text
home-library-api/
├── src/
│   ├── handlers.rs         # HTTP request/response logic and Axum extractors
│   ├── integration.rs      # Asynchronous external API proxy (Open Library)
│   ├── main.rs             # Application entry point, router, and middleware setup
│   ├── models.rs           # Data Transfer Objects (DTOs) and Serde serialization
│   └── repository.rs       # SQLx database operations and dynamic query builder
├── .env                    # Environment variables (Database URL)
├── .gitignore              # Version control exclusion rules
├── Cargo.lock              # Deterministic dependency tree
├── Cargo.toml              # Rust package and dependency configurations
├── CONTRIBUTING.md         # Engineering and pull request standards
├── docker-compose.yml      # Multi-container orchestration (API + DB)
├── Dockerfile              # Multi-stage build definition
└── README.md               # Project documentation
````

## Installation & Build

To deploy this architecture locally or on a homelab server, ensure you have **Docker** and **Docker Compose** installed.

```bash
# Clone the repository
git clone https://github.com/iago-fernandez/home-library-api.git
cd home-library-api

# Launch the infrastructure
docker compose up -d --build

# Optional: Clean up dangling builder images to save disk space
docker image prune -f
```

The API will be instantly available on `http://localhost:3000`.

## API Interface

### Standard Operations

* `GET /books`: Retrieve the inventory. Supports pagination (`limit`, `offset`) and sorting (`sort_by`, `sort_order`).
* `POST /books`: Register a new physical asset.
* `PUT /books/:id`: Update an existing record.
* `DELETE /books/:id`: Remove a record from the database.

### External Metadata Resolution

* `GET /books/lookup/:isbn`: Fetch precise book metadata utilizing the Open Library API.
* `GET /books/search-metadata?q=term`: Retrieve an array of potential book matches based on authors or titles.

## Advanced Query Engine Syntax

The `GET /books` endpoint supports appending operational suffixes to column names for complex filtering without requiring redundant endpoints:

* **Text Matching:** `_contains`, `_starts`, `_ends`, `_exact`
* **Numeric Evaluation:** `_gt`, `_gte`, `_lt`, `_lte`
* **Nullability:** `_empty=true` or `_empty=false`

*Example Request:*

```http
GET /books?title_starts=Harry&page_count_gte=200&personal_notes_empty=true
```

## Contributing

Contributions must adhere to strict idiomatic Rust standards and the Clean Code philosophy. Please review the [CONTRIBUTING.md](CONTRIBUTING.md) file for branching strategies and Conventional Commits guidelines.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.