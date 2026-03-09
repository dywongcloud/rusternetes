# Contributing to Rusternetes

Thank you for your interest in contributing to Rusternetes! This document provides guidelines and instructions for contributing.

## Development Setup

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed instructions on setting up your development environment.

Quick start:
```bash
./dev-setup.sh
```

## Before You Submit a PR

1. **Format your code:**
   ```bash
   make fmt
   ```

2. **Run the linter:**
   ```bash
   make clippy
   ```

3. **Run tests:**
   ```bash
   make test
   ```

4. **Or run all checks at once:**
   ```bash
   make pre-commit
   ```

## Development Workflow

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/YOUR_USERNAME/rusternetes.git
   cd rusternetes
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes**
   - Write code
   - Add tests
   - Update documentation

4. **Test your changes**
   ```bash
   # Run unit tests
   make test

   # Test in containerized environment
   make dev-full
   make kubectl-create-example-pod

   # Test locally
   make run-api-server  # In one terminal
   cargo run --bin kubectl -- --server http://localhost:6443 get pods
   ```

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

   Follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` - New feature
   - `fix:` - Bug fix
   - `docs:` - Documentation changes
   - `test:` - Test changes
   - `refactor:` - Code refactoring
   - `chore:` - Maintenance tasks

6. **Push and create a Pull Request**
   ```bash
   git push origin feature/your-feature-name
   ```

## Code Style

- Follow Rust standard naming conventions
- Use `rustfmt` for formatting (run `make fmt`)
- Use `clippy` for linting (run `make clippy`)
- Write clear, self-documenting code
- Add comments for complex logic
- Keep functions focused and small

## Testing Guidelines

- Write unit tests for new functionality
- Add integration tests where appropriate
- Ensure tests are deterministic
- Test error paths, not just happy paths
- Mock external dependencies in tests

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature_works() {
        // Arrange
        let input = setup_test_data();

        // Act
        let result = your_function(input).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

## Documentation

- Update README.md for user-facing changes
- Update DEVELOPMENT.md for development workflow changes
- Add inline documentation for public APIs
- Include examples for complex features
- Keep documentation up-to-date with code changes

## Pull Request Process

1. **Fill out the PR template** with:
   - Description of changes
   - Related issues
   - Testing performed
   - Screenshots (if UI changes)

2. **Ensure CI passes**:
   - All tests pass
   - Code is formatted
   - No clippy warnings

3. **Request review** from maintainers

4. **Address feedback** promptly

5. **Squash commits** if requested before merging

## Architecture Guidelines

When adding new features, follow Kubernetes architecture patterns:

- **API Server**: RESTful API, resource validation
- **Controllers**: Reconciliation loops, eventual consistency
- **Scheduler**: Resource-based pod placement
- **Kubelet**: Container lifecycle management
- **Storage**: etcd for persistent state

See [ARCHITECTURE.md](ARCHITECTURE.md) for details.

## Component-Specific Guidelines

### API Server
- Add new resources in `crates/common/src/resources/`
- Implement validation logic
- Add API endpoints in `crates/api-server/src/handlers/`
- Update OpenAPI specs (future)

### Scheduler
- Scheduling plugins go in `crates/scheduler/src/plugins/`
- Follow the plugin interface
- Add metrics for scheduling decisions

### Controller Manager
- New controllers in `crates/controller-manager/src/controllers/`
- Implement the `Controller` trait
- Use work queues for event processing

### Kubelet
- Container runtime logic in `crates/kubelet/src/runtime/`
- Pod lifecycle in `crates/kubelet/src/pod/`
- Node status updates in `crates/kubelet/src/status/`

## Common Tasks

### Adding a New Resource Type

1. Define the resource struct in `crates/common/src/resources/`
2. Implement serialization/deserialization
3. Add API endpoints in API server
4. Add controller logic if needed
5. Update kubectl to support the resource
6. Add examples to `examples/`

### Adding a New Controller

1. Create controller in `crates/controller-manager/src/controllers/`
2. Implement the `Controller` trait
3. Register controller in controller manager
4. Add tests
5. Document the controller's purpose

### Adding a New CLI Command

1. Add command in `crates/kubectl/src/commands/`
2. Update CLI parser
3. Add tests
4. Update documentation

## Performance Considerations

- Use async/await for I/O operations
- Avoid blocking the tokio runtime
- Use appropriate data structures
- Profile before optimizing
- Cache when appropriate, but invalidate correctly

## Security Considerations

- Validate all user inputs
- Use proper error handling (don't expose internals)
- Follow principle of least privilege
- Implement RBAC correctly
- Be cautious with unsafe code (avoid if possible)

## Getting Help

- Open an issue for bugs or feature requests
- Ask questions in discussions
- Check existing issues and PRs first
- Be respectful and follow the code of conduct

## License

By contributing to Rusternetes, you agree that your contributions will be licensed under the Apache-2.0 License.

## Thank You!

Your contributions make Rusternetes better for everyone. We appreciate your time and effort!
