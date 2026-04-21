# Contributing

Thank you for your interest in improving the Home Library API. This repository strictly adheres to modern Rust engineering standards, prioritizing memory safety, compile-time verification, and architectural cleanliness.

## Engineering Standards

To maintain a robust backend architecture, all contributions must adhere to the following:

* **Idiomatic Rust (Edition 2024):** Utilize modern language features such as let-chaining. Ensure all code passes `cargo fmt` and `cargo clippy` without warnings.
* **Clean Code Philosophy:** The code must be self-documenting through expressive variable naming, strict typing, and modularity. Avoid writing redundant comments that explain *what* the code does. Comments should only be used to explain *why* a highly complex or unusual architectural decision was made.
* **Error Handling:** Never use `.unwrap()` or `.expect()` in production business logic. Propagate errors gracefully using the `?` operator and map them to appropriate Axum HTTP Status Codes.
* **Separation of Concerns:** Maintain the structural integrity of the project. Network routing belongs in `handlers.rs`, database interactions in `repository.rs`, and external requests in `integration.rs`.

## Development Workflow

1. **Fork and Clone** the repository.
2. **Branching Strategy:**
   * `feat/`: New endpoints, query engine capabilities, or DTO expansions.
   * `fix/`: Resolution of SQL mapping errors, parsing bugs, or network timeouts.
   * `refactor/`: Code optimization, trait implementation, or dependency updates.
   * `docs/`: Content updates to technical documentation.
3. **Validation:** Ensure the project compiles flawlessly and passes all static analysis tools before pushing code.
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo check
   ```

## Commit Guidelines

We use **Conventional Commits** to maintain a clean, semantic, and easily traceable history:

* `feat(database): implement generic dynamic query builder`
* `fix(network): correct metadata proxy timeout handling`
* `refactor(core): optimize nested conditionals with let chaining`
* `chore(repository): update docker infrastructure`

## Pull Request Process

* Provide a clear, bulleted description of the modifications using the infinitive tense.
* Ensure your changes do not introduce new warnings or compilation errors.
* Squash intermediate commits to keep the repository history linear and atomic.