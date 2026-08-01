# Security Policy

## Supported Versions

Only the latest release or main branch of Familiar is actively supported for security updates.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Reporting a Vulnerability

We take the security of Familiar seriously. Because Familiar is a local-first application that handles sensitive developer context and local hook execution, security vulnerabilities (such as local privilege escalation, malicious hook payload execution, or sensitive data leakage) should be handled with care.

### How to Submit a Security Report

Please **do not** open a public GitHub issue for security vulnerabilities.

Instead, submit security reports by emailing the maintainers or using GitHub Private Vulnerability Reporting:

1. **GitHub Private Vulnerability Reporting**: Use the "Report a vulnerability" button on the GitHub repository Security tab.
2. **Email**: Contact maintainers via `glcoding@qq.com`.

Include as much of the following information as possible:
- Description of the issue and potential security impact.
- Step-by-step instructions or proof-of-concept script to reproduce the vulnerability.
- Affected component(s) (e.g. `familiar-hooks`, `familiar-core`, desktop API routes).
- Suggested remediation, if any.

### Response Expectations

- **Acknowledgment**: Within 48 hours of receiving your report.
- **Assessment**: Within 7 business days, confirming whether the report is valid and providing an estimated timeline for remediation.
- **Fix & Disclosure**: Once a fix is verified, a patch will be released, and an advisory will be published.
