# Security Checklist

This document outlines the security measures implemented in the PMP LLM Gateway API.

## Authentication & Authorization

| Feature | Status | Notes |
|---------|--------|-------|
| API Key Authentication | ✅ | SHA256 hashed keys with constant-time comparison |
| JWT Authentication | ✅ | For Admin UI, using jsonwebtoken crate |
| Role-Based Access | ✅ | Admin vs regular API keys |
| Resource Permissions | ✅ | All/Specific/None for models, prompts, KB |
| Rate Limiting | ✅ | Per-minute, per-hour, per-day limits |
| Key Expiration | ✅ | Configurable expiration timestamps |
| Key Revocation | ✅ | Immediate invalidation of compromised keys |

## Transport Security

| Feature | Status | Notes |
|---------|--------|-------|
| HTTPS Support | ✅ | Via reverse proxy (nginx, traefik) |
| HSTS Header | ✅ | `Strict-Transport-Security` header |
| TLS 1.2+ | ✅ | Configured at reverse proxy level |

## HTTP Security Headers

| Header | Status | Value |
|--------|--------|-------|
| X-Content-Type-Options | ✅ | `nosniff` |
| X-Frame-Options | ✅ | `DENY` |
| X-XSS-Protection | ✅ | `1; mode=block` |
| Referrer-Policy | ✅ | `strict-origin-when-cross-origin` |
| Content-Security-Policy | ✅ | API: `default-src 'none'`; UI: allows `'self'` + CDN scripts |
| Cache-Control | ✅ | `no-store, no-cache, must-revalidate` |

## Input Validation

| Feature | Status | Notes |
|---------|--------|-------|
| Request Size Limits | ✅ | 10 MB max body size |
| Path Traversal Prevention | ✅ | Blocks `..` and `//` in paths |
| Null Byte Injection | ✅ | Blocks null bytes in paths |
| JSON Schema Validation | ✅ | Via serde deserialization |
| ID Format Validation | ✅ | Alphanumeric + hyphens, max 50 chars |

## Data Protection

| Feature | Status | Notes |
|---------|--------|-------|
| Sensitive Data Redaction | ✅ | Logs redact passwords, tokens, API keys |
| Secure Header Logging | ✅ | Authorization headers logged as `[REDACTED]` |
| Password Hashing | ✅ | Argon2id for user passwords |
| Secret Storage | ⚠️ | Use K8s secrets, Vault, or AWS Secrets Manager |

## SQL Injection Prevention

| Feature | Status | Notes |
|---------|--------|-------|
| Parameterized Queries | ✅ | sqlx with compile-time checked queries |
| No Raw SQL | ✅ | All queries use parameter binding |

## Dependency Security

Run `cargo audit` regularly to check for vulnerabilities:

```bash
# Install cargo-audit
cargo install cargo-audit

# Run security audit
cargo audit
```

## Kubernetes Security

| Feature | Status | Notes |
|---------|--------|-------|
| Non-root User | ✅ | Runs as UID 1000 |
| Read-only Filesystem | ✅ | Uses emptyDir for /tmp |
| Dropped Capabilities | ✅ | `drop: ALL` |
| No Privilege Escalation | ✅ | `allowPrivilegeEscalation: false` |
| Resource Limits | ✅ | CPU and memory limits defined |
| Network Policies | ⚠️ | Implement based on your requirements |
| Pod Security Standards | ✅ | Restricted profile compatible |

## Secrets Management

### Recommended Practices

1. **Never commit secrets** to version control
2. **Use environment variables** from K8s secrets
3. **Rotate secrets regularly** especially after suspected compromise
4. **Use Vault or AWS Secrets Manager** for production

### Environment Variables

```bash
# Required secrets (via K8s Secret or similar)
APP__AUTH__JWT_SECRET=<secure-random-string>

# LLM Provider credentials
OPENAI_API_KEY=<key>
ANTHROPIC_API_KEY=<key>
```

## Logging Security

| Feature | Status | Notes |
|---------|--------|-------|
| No Secrets in Logs | ✅ | Sensitive fields redacted |
| Request ID Tracking | ✅ | For audit trails |
| Structured Logging | ✅ | JSON format for SIEM integration |
| Log Level Control | ✅ | Configurable via `APP__LOGGING__LEVEL` |

## Monitoring & Alerting

Configure alerts for:

1. **High error rates** (> 5% 5xx responses)
2. **Authentication failures** (multiple failed attempts)
3. **Rate limit hits** (potential abuse)
4. **Unusual traffic patterns** (potential DDoS)

## Security Recommendations

### Production Deployment

1. Use a reverse proxy (nginx, traefik) for TLS termination
2. Enable rate limiting at the ingress level as well
3. Implement network policies to restrict pod communication
4. Use a Web Application Firewall (WAF) if exposed publicly
5. Regular security scanning with tools like Trivy

### Regular Maintenance

1. Run `cargo audit` weekly
2. Update dependencies monthly
3. Review and rotate API keys quarterly
4. Conduct penetration testing annually

## Reporting Security Issues

Please report security vulnerabilities via private disclosure:
- Do NOT create public GitHub issues for security vulnerabilities
- Contact the maintainers directly with details
