# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability within DualMind, please send an email to [your-email@example.com](mailto:your-email@example.com). All security vulnerabilities will be promptly addressed.

Please do not disclose security vulnerabilities publicly until they have been handled by the maintainers.

## Security Measures

DualMind implements the following security measures:

1. **Environment Variables**: Sensitive information like API keys and endpoints are stored in environment variables, not in the code.
2. **Input Validation**: All user inputs are validated before processing.
3. **API Key Authentication**: API endpoints are protected with API key authentication.
4. **Rate Limiting**: API endpoints implement rate limiting to prevent abuse.

## Best Practices for Users

1. **Keep API Keys Secret**: Never share your API keys or include them in client-side code.
2. **Use Environment-Specific Configurations**: Create different configurations for development, testing, and production.
3. **Regularly Rotate API Keys**: Change your API keys periodically.
4. **Monitor API Usage**: Keep track of your API usage to detect unauthorized access.

## Secure Usage of Pre-built Executables

When using pre-built executables:

1. **Always create your own `.env` file**: Never use a pre-configured `.env` file from an untrusted source.
2. **Keep the executable and `.env` file in a secure location**: Ensure only authorized users can access them.
3. **Verify executable integrity**: Check the SHA256 hash of the downloaded executable against the published hash in the release notes.
4. **Do not run with elevated privileges**: The executable does not require administrator/root privileges to function. 