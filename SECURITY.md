# RetroMount Security Policy

## Supported Versions

Only the latest stable release of RetroMount is supported with security updates.

| Version | Supported |
| --- | --- |
| Latest | ✅ Yes |
| < 1.0 | ❌ No (Pre-release) |

---

## Reporting a Vulnerability

**Do not open a public issue** for security vulnerabilities. Instead:

1. **Email**: Send a detailed report to [lloydsmart@users.noreply.github.com](mailto:lloydsmart@users.noreply.github.com).
2. **Subject**: Use the prefix `[SECURITY] RetroMount`.
3. **Details**: Include the information below (see [Vulnerability Report Template](#vulnerability-report-template)).
4. **Encryption**: Use my [public GPG key](https://github.com/lloydsmart.gpg) (Key ID: `1534542E61DC82D3`) for sensitive details.

I will acknowledge receipt within **48 hours** and provide a timeline for a fix.

---

## Incident Response

### Triage

- Reports are reviewed within **24 hours** of receipt.
- Severity is classified as:
  - **Critical**: Remote code execution, privilege escalation.
  - **High**: Data leaks, authentication bypasses.
  - **Medium/Low**: Denial of service, minor information disclosure.

### Mitigation

- **Critical/High**: A patch is developed within **72 hours** and released as a hotfix.
- **Medium/Low**: Fixes are included in the next scheduled release.

### Communication

- Users are notified via GitHub Releases and the project’s issue tracker.
- Public disclosure occurs **30 days** after a patch, unless coordinated otherwise.

---

## Vulnerability Report Template

```plaintext
[Subject] [SECURITY] RetroMount: <Brief Description>

---
**Affected Version(s):**
<e.g., v1.2.0, main branch>

**Description:**
<Clear, concise steps to reproduce>

**Impact:**
<Potential consequences (e.g., data exposure, DoS)>

**Proof of Concept (if applicable):**
<Code snippets, logs, or screenshots>

**Suggested Fix:**
<Optional: Proposed patch or mitigation>

**Your Contact Info:**
<Name/Handle, Email, GPG Key (if encrypted)>
```

---

## Security Updates

- Patches are released as new versions on GitHub.
- All releases are GPG-signed (Key ID: `1534542E61DC82D3`).

---

## Best Practices for Contributors

- **Code Review**: All changes require maintainer approval.
- **Dependencies**: Audit with `cargo audit` before merging.
- **CI/CD**: Enforces Rustfmt, Clippy, and CodeQL scans.

---

## Scope

**In Scope:**

- RetroMount codebase
- Official Debian packages
- GitHub Actions workflows

**Out of Scope:**

- Third-party dependencies
- User misconfigurations

---

## GPG-Signed Releases

Verify releases with:

```bash
gpg --verify retromount-vX.Y.Z.tar.gz.asc
```

---

## Acknowledgements

Responsible disclosures are credited in `THANKS.md`.

---

**Contact:** [lloydsmart@users.noreply.github.com](mailto:lloydsmart@users.noreply.github.com)
