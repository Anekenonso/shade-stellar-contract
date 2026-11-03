# Security Policy

Shade Protocol takes the security of its decentralized payment gateway, smart contracts, and user/merchant funds seriously. This document defines the protocol's security policy, supported versions, private vulnerability reporting guidelines, coordinated disclosure procedures, live incident response mechanisms, and safe harbor protections for good-faith security researchers.

---

## 1. Supported Versions & Scope

Security updates, audits, and fixes are actively maintained for the current deployment baseline running on Stellar Network.

### In-Scope Smart Contracts (`contracts/`)

| Contract | Path | Role & Assets Handled |
| :--- | :--- | :--- |
| **`shade`** | `contracts/shade` | Central payment gateway hub, invoice routing, fee management, token whitelisting |
| **`account`** | `contracts/account` | Dedicated merchant settlement and withdrawal accounts, withdrawal limits |
| **`escrow`** | `contracts/escrow` | Conditional buyer-merchant milestone escrows, timelocks, dispute release |
| **`escrow_factory`** | `contracts/escrow_factory` | Deterministic deployment and tracking of merchant escrow instances |
| **`subscription`** | `contracts/subscription` | Recurring payment authorization, billing schedules, allowance tracking |
| **`ticketing`** | `contracts/ticketing` | Event ticketing issuance, resale price caps, check-in validation |
| **`ticketing_factory`**| `contracts/ticketing_factory`| Factory registry for merchant ticketing contracts |
| **`crowdfund`** | `contracts/crowdfund` | Milestone-based funding campaigns, pledges, refund logic |
| **`crowdfund_factory`**| `contracts/crowdfund_factory`| Factory registry for crowdfund campaign contracts |

### In-Scope Deployments & Networks

- **Mainnet**: Production contract instances initialized with multisig administration.
- **Testnet**: Official testnet instances verifying staging releases.
- **Futurenet**: Protocol testing against Soroban future releases.

### Out of Scope

- Third-party Stellar RPC nodes, Horizon infrastructure, or network-level validator consensus failures.
- User operational errors, compromised merchant private keys, or phishing attacks.
- Theoretical attacks requiring impossible compute power or non-standard Soroban VM host modifications.
- Vulnerabilities in upstream Soroban / Stellar SDK dependencies already publicly tracked by the Stellar Development Foundation (unless an exploitable interaction specifically exists within Shade Protocol).

---

## 2. Reporting a Vulnerability

**DO NOT open public GitHub issues, discussions, or pull requests for suspected security vulnerabilities.** Public disclosure puts merchant and customer funds at risk.

Please report all security vulnerabilities through one of the following private channels:

### Primary Channel: GitHub Private Vulnerability Reporting (Recommended)
1. Navigate to the **[Security tab](https://github.com/ShadeProtocol/shade-stellar-contract/security)** of this repository.
2. Under "Vulnerability disclosure", click **"Report a vulnerability"** to open a confidential advisory draft.
3. Provide full reproduction steps, affected contract functions, and proof-of-concept (PoC) code.

### Alternative Channel: Security Email
If GitHub Private Vulnerability Reporting is unavailable, email our security team directly:
- **Email**: `security@shadeprotocol.io`
- **Subject**: `[VULNERABILITY] <Component/Contract> - <Brief Severity/Description>`
- **PGP Encryption**: For highly sensitive reports, please encrypt your email using the protocol security team PGP public key (fingerprint available on request or in release announcements).

### Report Contents Checklist
To expedite triage and resolution, please include:
- Contract name, function name, and source file path(s).
- Detailed description of the vulnerability (e.g., access control bypass, arithmetic error, reentrancy, re-entrancy / state desync, replay attack).
- Step-by-step reproduction or a standalone Rust/Soroban test case showing the exploit.
- Threat impact assessment (potential funds at risk, denial of service, unauthorized role grant).
- Suggested remediation or patch if available.

---

## 3. Response Timelines & SLAs

The Shade Protocol core team follows a strict Service Level Agreement (SLA) for vulnerability reports:

| Phase | Target Timeline | Details |
| :--- | :--- | :--- |
| **Initial Acknowledgment** | **Within 24 hours** | Receipt confirmation and assignment of an internal lead engineer. |
| **Triage & Severity Classification** | **Within 72 hours** | Reproduction of the issue, CVSS scoring, and scope confirmation. |
| **Fix Development & Testing** | **3 to 14 days** | Engineering remediation, regression tests, and internal peer review. |
| **Upgrade Deployment / Release** | **7 to 30 days** | Staging verification, governance timelock (if applicable), and mainnet upgrade. |
| **Coordinated Disclosure** | **Up to 90 days** | Public post-mortem and release notes published in coordination with the researcher. |

---

## 4. Emergency Response & Live Incident Mitigation

If a live critical vulnerability or active exploit is detected, the protocol team activates the following emergency procedures:

1. **Emergency Pause (`Pausable`)**:
   - The protocol emergency admins can invoke `pause_shade()` on the central hub to immediately halt invoice creation, payments, and account withdrawals.
   - For complete details on paused functions, operational playbooks, and resumption procedures, see the [Pausable Emergency-Stop Guide](docs/security/pausable.md).

2. **Access Control & Role Revocation**:
   - Compromised operational roles can be immediately revoked via `revoke_role()`. Refer to [Access Control & Roles](docs/security/access-control.md) and [Admin & Ownership](docs/security/admin-and-ownership.md).

3. **Timelocked Parameter & Fee Safeguards**:
   - Fee changes require a two-step timelock proposal (`propose_fee` / `execute_fee`), preventing sudden fee hijacking.

4. **WASM Contract Upgrades**:
   - Emergency contract upgrades follow the audited upgrade pattern using Soroban bytecode replacement. Detailed execution steps are documented in the [Upgrade Runbook](docs/operations/upgrade-runbook.md) and [Admin Operations Runbook](docs/operations/admin-runbook.md).

---

## 5. Bug Bounty Position & Recognition

Shade Protocol values the contributions of ethical security researchers.

- **Bounty Eligibility**: Valid, original vulnerabilities that have not been previously reported and are disclosed responsibly in compliance with this policy are eligible for recognition and reward.
- **Compensation**: Bounties are evaluated on a severity-adjusted scale (aligned with CVSS) and awarded in USDC / XLM via on-chain settlement:
  - **Critical** (Direct loss of merchant or escrow funds, permanent contract lock): Up to $5,000+ USD
  - **High** (Temporary freeze of funds, unauthorized state alteration, fee circumvention): Up to $2,500 USD
  - **Medium** (Griefing, denial of service requiring admin reset, logic edge-cases): Up to $1,000 USD
  - **Low** (Informational, minor event inconsistencies, non-exploitable edge cases): Hall of Fame credit / swag
- **Hall of Fame**: Disclosers will be credited in our public security advisories, release notes, and contributor hall of fame (unless anonymity is requested).

---

## 6. Safe Harbor Policy

Shade Protocol considers security research conducted in good faith to be authorized. We commit not to pursue legal action against researchers, provided that you:

1. **Act in Good Faith**: Avoid privacy violations, destruction of data, and interruption or degradation of our services or user accounts.
2. **Do Not Exploit Live Funds**: Do not extract or siphon funds from live mainnet accounts or contracts. All testing should be executed against local Soroban test harnesses, sandbox environments, or Futurenet/Testnet.
3. **Keep Information Confidential**: Maintain confidentiality regarding discovered vulnerabilities until coordinated public disclosure is agreed upon.
4. **Follow Disclosure Guidelines**: Report issues promptly via the private channels listed above.

If your research complies with these rules, we consider your research authorized and will work with you to understand and resolve the issue quickly.

---

## 7. Repository Settings Notice for Maintainers

To enable GitHub's built-in private reporting interface:
1. Go to repository **Settings** > **Code security and analysis**.
2. Under "Vulnerability reporting", click **Enable** next to **Private vulnerability reporting**.
3. Security researchers will then see the "Report a vulnerability" button under the repository's Security tab.
