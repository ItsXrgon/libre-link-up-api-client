# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please follow these steps:

1. **Do Not** open a public issue
2. Email the maintainers directly at: [ali.koheil@gmail.com]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We aim to respond within 48 hours and will work with you to understand and address the issue promptly.

## Security Considerations

This library handles sensitive health data and authentication credentials. When using this library:

1. **Never** commit credentials to version control
2. Use environment variables or secure credential storage
3. Be aware of API rate limiting
4. This is an **unofficial** client - the API may change without notice
5. Ensure you comply with all relevant data protection regulations (HIPAA, GDPR, etc.)

## Best Practices

- Always use HTTPS (enforced by default)
- Store credentials securely (use environment variables)
- Regularly update dependencies: `cargo update`
- Review the changelog for security-related updates
- Use the latest stable Rust version

## Dependencies

This project uses several dependencies. We monitor them for security vulnerabilities:

- Run `cargo audit` regularly to check for known vulnerabilities
- Dependencies are kept up to date with compatible versions

## Disclosure Policy

When a security issue is reported and fixed:

1. We will release a patch version
2. Document the fix in the changelog
3. Credit the reporter (if desired)
4. Publish a security advisory if necessary
