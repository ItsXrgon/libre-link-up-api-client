# Contributing to LibreLinkUp API Client

Thank you for your interest in contributing! This document provides guidelines for contributing to this project.

## Code of Conduct

This project follows the Rust Code of Conduct. Please be respectful and professional in all interactions.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in Issues
2. Create a new issue with a clear title and description
3. Include steps to reproduce, expected behavior, and actual behavior
4. Add relevant code samples or error messages

### Suggesting Enhancements

1. Check if the enhancement has already been suggested
2. Create a new issue describing the enhancement
3. Explain why this enhancement would be useful

### Pull Requests

1. Fork the repository
2. Create a new branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cargo test`
5. Run formatting: `cargo fmt`
6. Run linting: `cargo clippy`
7. Commit with clear messages
8. Push to your fork
9. Create a Pull Request

## Development Setup

```bash
# Clone the repository
git clone https://github.com/YOURNAME/libre_link_up_api_client
cd libre_link_up_api_client

# Build the project
cargo build

# Run tests
cargo test

# Run examples (requires credentials)
cargo run --example basic_usage
```

## Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Address all clippy warnings (`cargo clippy`)
- Write doc comments for public APIs
- Add tests for new features

## Testing

- Unit tests go in the same file as the code
- Integration tests go in the `tests/` directory
- Mark real API tests with `#[ignore]` to avoid rate limiting

## Documentation

- Update README.md for user-facing changes
- Update CHANGELOG.md following Keep a Changelog format
- Write doc comments with examples

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
