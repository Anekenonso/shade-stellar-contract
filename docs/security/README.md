# Security

Threat model, access control, and audit-relevant notes for the Shade Protocol contracts.

- [Security Policy & Vulnerability Disclosure](../../SECURITY.md) — supported versions, private reporting channels, SLAs, bug bounty guidelines, and safe harbor.
- [Threat model and security assumptions](threat-model.md) — assets, trust assumptions, attacker classes, attack surfaces, and known limitations.
- [Signed invoices, merchant keys, and signature verification](signatures.md) — off-chain signing scheme, nonce replay protection, and key rotation.
- [Access control and roles](access-control.md) — admin supremacy, Role enum, grant/revoke, permission matrix.
- [Pausable emergency-stop mechanism](pausable.md) — pause/unpause, blocked/allowed functions, operational playbook.
- [Reentrancy protection](reentrancy.md) — enter/exit guard, guarded functions, contributor rules.
- [Admin ownership and two-step admin transfer](admin-and-ownership.md) — initialization, propose/accept flow, key management.

← [Back to documentation home](../README.md)
