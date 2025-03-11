# Contributing to DualMind

Thank you for considering contributing to DualMind! This document outlines the guidelines for contributing to the project.

## Security Guidelines

1. **Never commit sensitive information**: This includes API keys, passwords, and internal URLs.
2. **Use environment variables**: All sensitive configuration should be loaded from environment variables.
3. **Review code for security issues**: Before submitting a PR, review your code for potential security issues.
4. **Report security vulnerabilities privately**: If you discover a security vulnerability, please report it privately as outlined in our [Security Policy](SECURITY.md).

## Development Setup

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/dualmind.git`
3. Create a new branch: `git checkout -b feature/your-feature-name`
4. Copy `.env.example` to `.env` and add your own configuration
5. Make your changes
6. Run tests: `cargo test`
7. Submit a pull request

## Code Style

- Follow the Rust style guidelines
- Write clear, concise commit messages
- Include tests for new features
- Update documentation as needed

## Pull Request Process

1. Ensure your code passes all tests
2. Update the README.md with details of changes if applicable
3. The PR will be merged once it has been reviewed and approved by a maintainer 