# Security Policy

## Supported Versions

We release patches for security vulnerabilities in the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

**Note:** We recommend always using the latest stable version to ensure you have all security updates and patches.

## Reporting a Vulnerability

We take the security of RoomRTC seriously. If you believe you have found a security vulnerability, please report it to us as described below.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report security vulnerabilities by:

1. **Email**: Send an email to sebita29@gmail.com
2. **Subject Line**: Include "RoomRTC Security Vulnerability" in the subject
3. **Details to Include**:
   - Type of vulnerability
   - Full paths of source file(s) related to the vulnerability
   - Location of the affected source code (tag/branch/commit or direct URL)
   - Step-by-step instructions to reproduce the issue
   - Proof-of-concept or exploit code (if possible)
   - Impact of the vulnerability, including how an attacker might exploit it

### What to Expect

- **Acknowledgment**: You should receive an acknowledgment within 48 hours
- **Investigation**: We will investigate and validate the reported vulnerability
- **Updates**: We will keep you informed about the progress of the fix
- **Resolution**: Once the vulnerability is fixed, we will notify you and publicly acknowledge your responsible disclosure (unless you prefer to remain anonymous)
- **Timeline**: We aim to resolve critical vulnerabilities within 30 days

### Disclosure Policy

- Please give us reasonable time to address the vulnerability before any public disclosure
- We will credit you for the discovery in our security advisories (unless you prefer anonymity)
- We follow coordinated disclosure practices

## Security Best Practices

### For Users

1. **Keep Updated**: Always use the latest stable version of RoomRTC
2. **Secure Configuration**:
   - Use strong, unique passwords for any authentication
   - Enable HTTPS/WSS for all WebRTC connections
   - Configure proper CORS policies
3. **Network Security**:
   - Use TURN servers with authentication
   - Implement proper firewall rules
   - Validate and sanitize all user inputs
4. **Monitoring**: 
   - Monitor your application logs for suspicious activity
   - Set up alerts for unusual connection patterns

### For Developers

1. **Secure Coding Practices**:
   - Validate and sanitize all inputs, especially signaling data
   - Use parameterized queries to prevent injection attacks
   - Implement proper authentication and authorization
   - Follow the principle of least privilege

2. **WebRTC Security**:
   - Always use HTTPS for serving WebRTC applications
   - Use WSS (WebSocket Secure) for signaling
   - Implement proper STUN/TURN server authentication
   - Validate peer connections before establishing media streams

3. **Dependencies**:
   - Regularly update all dependencies
   - Use tools like `npm audit` or `yarn audit` to check for known vulnerabilities
   - Review dependency licenses and security advisories

4. **Data Protection**:
   - Implement end-to-end encryption for sensitive data
   - Don't store sensitive information in logs
   - Follow GDPR and other relevant data protection regulations
   - Use secure token generation for session management

5. **Code Review**:
   - Conduct security-focused code reviews
   - Use static analysis tools to identify potential vulnerabilities
   - Implement automated security testing in CI/CD pipelines

### Security Checklist

Before deploying RoomRTC in production:

- [ ] All communications use HTTPS/WSS
- [ ] Authentication is properly implemented
- [ ] Input validation is in place for all user inputs
- [ ] CORS policies are correctly configured
- [ ] Rate limiting is implemented to prevent DoS attacks
- [ ] Error messages don't expose sensitive information
- [ ] Security headers are properly configured
- [ ] Dependencies are up to date
- [ ] Logging doesn't include sensitive data
- [ ] Security testing has been performed

## Known Security Considerations

### WebRTC Specific

1. **IP Address Exposure**: WebRTC can expose users' real IP addresses even when using a VPN. Consider:
   - Implementing IP masking via TURN servers
   - Warning users about potential IP exposure
   - Providing configuration options for privacy-conscious users

2. **Cross-Site Scripting (XSS)**: Ensure all user-generated content is properly sanitized

3. **Man-in-the-Middle Attacks**: Always use encrypted connections and verify peer identities

## Security Updates

Security updates and patches will be announced through:
- GitHub Security Advisories
- Release notes
- Project README

Subscribe to repository releases to stay informed about security updates.

## Additional Resources

- [WebRTC Security Architecture](https://datatracker.ietf.org/doc/html/rfc8827)
- [OWASP WebRTC Security Guidelines](https://owasp.org/)
- [MDN Web Security](https://developer.mozilla.org/en-US/docs/Web/Security)

---

**Last Updated**: 2025-12-17

Thank you for helping keep RoomRTC and its users safe!
