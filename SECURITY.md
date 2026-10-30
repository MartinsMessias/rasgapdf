# Security Policy

The RasgaPDF project takes supply chain security, data privacy, and vulnerability management seriously.

## Supported Versions

Only the latest release of RasgaPDF receives security patches and vulnerability updates.

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| < 0.3.0 | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability or potential supply-chain threat in **RasgaPDF** (core engine, FFI, or language bindings), please do **NOT** open a public issue.

Instead, please report it via one of the following secure channels:

1. **GitHub Private Vulnerability Reporting (Recommended):**
   - Go to the [Security Advisories](https://github.com/MartinsMessias/rasgapdf/security/advisories) tab of this repository.
   - Click **"Report a vulnerability"** to open a private advisory draft.

2. **Direct Security Contact:**
   - Email **Messias Martins** at: [`messiasads2017@gmail.com`](mailto:messiasads2017@gmail.com?subject=RasgaPDF%20Security%20Vulnerability%20Report)
   - Please include:
     - Type of vulnerability (e.g., buffer overflow, memory safety, dependency issue, injection).
     - Step-by-step reproduction instructions or a minimal reproducible PDF.
     - Affected versions and environments (OS, architecture, runtime).

### Response SLA & Disclosure Process

- **Initial Response:** Within 48 hours of receipt.
- **Assessment & Triage:** Within 5 business days.
- **Patch & Release:** Fixes will be prepared in a private fork and released alongside a GitHub Security Advisory (GHSA) and CVE identifier when applicable.
- **Coordination:** We adhere to responsible coordinated disclosure guidelines.

## Supply Chain Integrity & Provenance

RasgaPDF releases are cryptographically signed and verifiable:
- **PyPI Attestations:** Built via GitHub Actions with OpenID Connect (OIDC) trusted publishing and digital attestations.
- **npm Sigstore Provenance:** Published with `--provenance` recorded into the Sigstore public transparency log.
- **Static Analysis (SAST):** Scanned continuously with GitHub CodeQL and OpenSSF Scorecard.
- **Dependency Automation:** Tracked via Dependabot for Rust (Cargo), Python (PyPI), and Node.js (npm).
