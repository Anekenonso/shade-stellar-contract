# Protocol Architecture & Flow Diagrams

This gallery compiles the formal Mermaid sequence diagrams, state-machine diagrams, and component interaction maps for Shade Protocol. Every diagram adheres to GitHub-compatible Mermaid syntax and models on-chain Soroban contract interactions, actors, and token transfers.

---

## Table of Contents

- [1. Component Architecture Diagram](#1-component-architecture-diagram)
- [2. Sequence Diagrams](#2-sequence-diagrams)
  - [2.1 Merchant Registration & Account Deployment](#21-merchant-registration--account-deployment)
  - [2.2 Invoice Creation & Payment with Fee Routing](#22-invoice-creation--payment-with-fee-routing)
  - [2.3 Partial Payment Settlement](#23-partial-payment-settlement)
  - [2.4 Refund Processing](#24-refund-processing)
  - [2.5 Fiat Invoice Resolution via Oracle](#25-fiat-invoice-resolution-via-oracle)
  - [2.6 Recurring Subscription Charge](#26-recurring-subscription-charge)
  - [2.7 Escrow Deposit & Milestone Release](#27-escrow-deposit--milestone-release)
  - [2.8 Ticket Purchase & Capped Resale](#28-ticket-purchase--capped-resale)
  - [2.9 Two-Step Contract Upgrade Flow](#29-two-step-contract-upgrade-flow)
- [3. State Transition Diagrams](#3-state-transition-diagrams)
  - [3.1 Invoice Status Lifecycle](#31-invoice-status-lifecycle)
  - [3.2 Subscription Status Lifecycle](#32-subscription-status-lifecycle)
  - [3.3 Escrow Status Lifecycle](#33-escrow-status-lifecycle)
  - [3.4 Merchant Withdrawal Request Status](#34-merchant-withdrawal-request-status)
  - [3.5 Ticket Token Lifecycle](#35-ticket-token-lifecycle)

---

## 1. Component Architecture Diagram

The diagram below illustrates the cross-contract invocation topography between the central `shade` payment hub and modular satellite contracts in the Soroban host environment.

```mermaid
graph TB
    subgraph External["External Actors & Infrastructure"]
        Merchant["Merchant"]
        Customer["Customer / Payer"]
        Keeper["Subscription Keeper"]
        Oracle["Price Oracle (SEP-40)"]
    end

    subgraph Hub["Core Gateway"]
        ShadeContract["shade (Payment Hub & Router)"]
    end

    subgraph Satellites["Modular Domain Contracts"]
        AccountContract["account (Dedicated Merchant Account)"]
        EscrowFactory["escrow_factory"]
        EscrowContract["escrow"]
        SubContract["subscription"]
        TicketFactory["ticketing_factory"]
        TicketContract["ticketing"]
        CrowdfundFactory["crowdfund_factory"]
        CrowdfundContract["crowdfund"]
    end

    subgraph TokenEngine["Stellar Asset Subsystem"]
        StellarToken["Stellar Asset Contract (SAC / SEP-41)"]
        Treasury["Protocol Treasury Account"]
    end

    %% Invocations
    Merchant -->|Register / Invoicing| ShadeContract
    Customer -->|Pay Invoice| ShadeContract
    Keeper -->|Trigger Billing| SubContract
    ShadeContract -->|Deploy / Withdraw| AccountContract
    ShadeContract -->|Query Rates| Oracle
    ShadeContract -->|Transfer SAC| StellarToken
    StellarToken -->|Deduct Fee| Treasury
    StellarToken -->|Net Settlement| AccountContract

    %% Factories
    EscrowFactory -->|Deploy Instance| EscrowContract
    TicketFactory -->|Deploy Instance| TicketContract
    CrowdfundFactory -->|Deploy Instance| CrowdfundContract

    %% Interactions
    Customer -->|Deposit / Escrow| EscrowContract
    Customer -->|Purchase Ticket| TicketContract
    Customer -->|Authorize / Pay| SubContract
    EscrowContract -.->|Fee Callback| ShadeContract
```

---

## 2. Sequence Diagrams

### 2.1 Merchant Registration & Account Deployment

Shows how a merchant registers on the hub, resulting in the deterministic deployment and binding of an isolated `account` contract.

```mermaid
sequenceDiagram
    autonumber
    actor Merchant
    participant Shade as shade (Hub)
    participant Account as account (New Instance)
    participant Storage as Contract Storage

    Merchant->>Shade: register_merchant(payout_address, metadata)
    activate Shade
    Shade->>Shade: require_auth(Merchant)
    Shade->>Shade: check_not_registered(Merchant)
    Shade->>Account: deploy(WASM_HASH, salt=Merchant)
    activate Account
    Account->>Account: initialize(merchant=Merchant, hub=Shade)
    Account-->>Shade: account_contract_id
    deactivate Account
    Shade->>Storage: write(DataKey::Merchant(Merchant), {account_id, active: true})
    Shade-->>Merchant: registration_confirmed(account_contract_id)
    deactivate Shade
```

---

### 2.2 Invoice Creation & Payment with Fee Routing

Shows standard payment routing: token transfer authorization, fee deduction routed to the protocol treasury, and net settlement in the merchant's dedicated account.

```mermaid
sequenceDiagram
    autonumber
    actor Customer as Customer / Payer
    participant Shade as shade (Hub)
    participant Token as Stellar Token (SAC)
    participant Account as Merchant Account
    participant Treasury as Protocol Treasury

    Customer->>Shade: pay_invoice(invoice_id, token, amount)
    activate Shade
    Shade->>Shade: require_auth(Customer)
    Shade->>Shade: validate_invoice_status(invoice_id, amount)
    Shade->>Shade: calculate_fee(amount, merchant_volume_tier)
    
    Note over Shade: Fee = fee_bps * amount / 10000<br/>Net = amount - Fee

    Shade->>Token: transfer_from(Customer, Treasury, Fee)
    activate Token
    Token-->>Shade: ok
    deactivate Token

    Shade->>Token: transfer_from(Customer, Account, Net)
    activate Token
    Token-->>Shade: ok
    deactivate Token

    Shade->>Account: record_deposit(invoice_id, token, Net)
    activate Account
    Account-->>Shade: balance_updated
    deactivate Account

    Shade->>Shade: update_invoice_status(PAID)
    Shade-->>Customer: payment_receipt(invoice_id, tx_hash)
    deactivate Shade
```

---

### 2.3 Partial Payment Settlement

Shows installment or partial invoice payments updating remaining balances without prematurely marking invoices fully paid.

```mermaid
sequenceDiagram
    autonumber
    actor Customer
    participant Shade as shade (Hub)
    participant Token as Stellar Token
    participant Storage as Invoice Storage

    Customer->>Shade: pay_partial(invoice_id, partial_amount)
    activate Shade
    Shade->>Shade: require_auth(Customer)
    Shade->>Storage: read(invoice_id)
    Storage-->>Shade: {total: 100, paid: 30, status: PARTIAL}
    
    Shade->>Token: transfer_from(Customer, Account, partial_amount)
    Token-->>Shade: ok

    Shade->>Shade: new_paid = 30 + partial_amount (e.g., 40 -> 70)
    alt new_paid < total
        Shade->>Storage: write(invoice_id, {paid: 70, status: PARTIAL})
        Shade-->>Customer: partial_receipt(remaining: 30)
    else new_paid >= total
        Shade->>Storage: write(invoice_id, {paid: 100, status: PAID})
        Shade-->>Customer: invoice_completed()
    end
    deactivate Shade
```

---

### 2.4 Refund Processing

Shows merchant-initiated refunds returning tokens back to the customer's wallet.

```mermaid
sequenceDiagram
    autonumber
    actor Merchant
    participant Shade as shade (Hub)
    participant Account as Merchant Account
    participant Token as Stellar Token
    actor Customer

    Merchant->>Shade: refund_invoice(invoice_id, refund_amount, reason)
    activate Shade
    Shade->>Shade: require_auth(Merchant)
    Shade->>Shade: verify_invoice_merchant(invoice_id, Merchant)
    Shade->>Account: authorize_refund(token, refund_amount)
    activate Account
    Account->>Token: transfer(Account, Customer, refund_amount)
    activate Token
    Token-->>Account: ok
    deactivate Token
    Account-->>Shade: refund_debited
    deactivate Account
    Shade->>Shade: record_refund_event(invoice_id, refund_amount)
    Shade-->>Merchant: refund_success(invoice_id)
    deactivate Shade
```

---

### 2.5 Fiat Invoice Resolution via Oracle

Illustrates how an invoice denominated in fiat (e.g., USD) dynamically resolves to accepted Stellar tokens using a SEP-40 price oracle feed.

```mermaid
sequenceDiagram
    autonumber
    actor Customer
    participant Shade as shade (Hub)
    participant Oracle as Price Oracle (SEP-40)
    participant Token as Stellar Token (USDC/XLM)

    Customer->>Shade: get_invoice_token_amount(invoice_id, token_address)
    activate Shade
    Shade->>Shade: read_fiat_price(invoice_id) -> $50.00 USD
    Shade->>Oracle: lastprice(Base=USD, Quote=Token)
    activate Oracle
    Oracle-->>Shade: price_data (timestamp, decimals, price=0.125)
    deactivate Oracle
    Shade->>Shade: verify_staleness_threshold(timestamp <= 300s)
    Shade->>Shade: required_tokens = fiat_amount / oracle_price
    Shade-->>Customer: quote(required_tokens, expiry_timestamp)
    deactivate Shade
```

---

### 2.6 Recurring Subscription Charge

Models automated recurring subscription billing triggered by keepers using user pre-authorized allowances.

```mermaid
sequenceDiagram
    autonumber
    actor Keeper as Billing Keeper
    participant SubContract as subscription
    participant Token as Stellar Token
    participant Account as Merchant Account
    actor Subscriber

    Keeper->>SubContract: charge_subscription(subscription_id)
    activate SubContract
    SubContract->>SubContract: verify_interval_elapsed(now >= next_billing_ts)
    SubContract->>SubContract: verify_allowance_valid(Subscriber, max_amount)
    
    SubContract->>Token: transfer_from(Subscriber, Account, charge_amount)
    activate Token
    Token-->>SubContract: ok
    deactivate Token

    SubContract->>SubContract: update_next_billing_ts(next_billing_ts + interval)
    SubContract->>SubContract: emit_event(SubscriptionCharged)
    SubContract-->>Keeper: charge_confirmed()
    deactivate SubContract
```

---

### 2.7 Escrow Deposit & Milestone Release

Shows the multi-party conditional escrow lifecycle from initial funding to milestone verification and dispute arbitration.

```mermaid
sequenceDiagram
    autonumber
    actor Buyer
    participant Escrow as escrow
    actor Seller
    actor Arbiter

    Buyer->>Escrow: deposit_and_lock(milestone_id, amount)
    activate Escrow
    Escrow->>Escrow: require_auth(Buyer)
    Escrow->>Escrow: status = FUNDED
    deactivate Escrow

    Seller->>Escrow: submit_milestone_delivery(milestone_id, proof_uri)
    
    alt Buyer Approves
        Buyer->>Escrow: approve_release(milestone_id)
        activate Escrow
        Escrow->>Escrow: transfer_to(Seller, amount)
        Escrow->>Escrow: status = RELEASED
        deactivate Escrow
    else Dispute Raised
        Buyer->>Escrow: raise_dispute(milestone_id)
        activate Escrow
        Escrow->>Escrow: status = DISPUTED
        deactivate Escrow
        Arbiter->>Escrow: resolve_dispute(milestone_id, split_buyer, split_seller)
        activate Escrow
        Escrow->>Escrow: transfer_to(Buyer, split_buyer)
        Escrow->>Escrow: transfer_to(Seller, split_seller)
        Escrow->>Escrow: status = RESOLVED
        deactivate Escrow
    end
```

---

### 2.8 Ticket Purchase & Capped Resale

Illustrates anti-scalping logic enforcing hard resale price ceilings on secondary ticket transfers.

```mermaid
sequenceDiagram
    autonumber
    actor Buyer
    participant Ticket as ticketing
    actor OriginalOwner as Primary Ticket Holder

    Note over Ticket: Primary Issuance: price = 50 USDC, max_resale_cap = 60 USDC

    OriginalOwner->>Ticket: list_for_resale(ticket_id, asking_price=55)
    activate Ticket
    Ticket->>Ticket: assert(asking_price <= max_resale_cap)
    Ticket->>Ticket: set_status(LISTED)
    deactivate Ticket

    Buyer->>Ticket: buy_resale_ticket(ticket_id)
    activate Ticket
    Ticket->>Ticket: require_auth(Buyer)
    Ticket->>Ticket: transfer_funds(Buyer, OriginalOwner, 55)
    Ticket->>Ticket: transfer_token_id(OriginalOwner, Buyer)
    Ticket->>Ticket: set_status(ACTIVE)
    Ticket-->>Buyer: ticket_ownership_transferred()
    deactivate Ticket
```

---

### 2.9 Two-Step Contract Upgrade Flow

Shows how audited WASM bytecode replaces existing contract logic while safeguarding persistent storage schemas.

```mermaid
sequenceDiagram
    autonumber
    actor Admin
    participant Shade as shade (Hub)
    participant Host as Soroban Runtime

    Admin->>Host: stellar contract install --wasm new_shade.optimized.wasm
    activate Host
    Host-->>Admin: new_wasm_hash (32 bytes)
    deactivate Host

    Admin->>Shade: propose_upgrade(new_wasm_hash)
    activate Shade
    Shade->>Shade: require_auth(Admin)
    Shade->>Shade: record_timelock_proposal(new_wasm_hash, unlock_time)
    deactivate Shade

    Note over Shade: Timelock delay elapses (e.g. 48 hours)

    Admin->>Shade: execute_upgrade()
    activate Shade
    Shade->>Shade: verify_timelock_ready()
    Shade->>Host: update_current_contract_wasm(new_wasm_hash)
    activate Host
    Host-->>Shade: bytecode_updated
    deactivate Host
    Shade->>Shade: emit_event(ContractUpgraded)
    Shade-->>Admin: upgrade_complete()
    deactivate Shade
```

---

## 3. State Transition Diagrams

### 3.1 Invoice Status Lifecycle

```mermaid
stateDiagram-v2
    [*] --> DRAFT : create_invoice()
    DRAFT --> PENDING : publish_invoice()
    
    PENDING --> PARTIAL : pay_partial() [amount < total]
    PARTIAL --> PARTIAL : pay_partial() [accumulated < total]
    PARTIAL --> PAID : pay_partial() [accumulated >= total]
    
    PENDING --> PAID : pay_full()
    
    PENDING --> VOIDED : void_invoice() [by Merchant]
    PARTIAL --> VOIDED : void_invoice() [with refund]
    
    PENDING --> EXPIRED : block_time > expiry_timestamp
    PARTIAL --> EXPIRED : block_time > expiry_timestamp
    
    PAID --> REFUNDED : refund_invoice() [full or partial]
    
    PAID --> [*]
    VOIDED --> [*]
    EXPIRED --> [*]
    REFUNDED --> [*]
```

---

### 3.2 Subscription Status Lifecycle

```mermaid
stateDiagram-v2
    [*] --> PENDING_AUTH : subscribe(tier, interval)
    PENDING_AUTH --> ACTIVE : grant_allowance() + initial_charge()
    
    ACTIVE --> ACTIVE : charge_subscription() [successful]
    
    ACTIVE --> PAST_DUE : charge_subscription() [insufficient funds]
    PAST_DUE --> ACTIVE : retry_charge() [succeeds within grace period]
    PAST_DUE --> SUSPENDED : grace_period_expired
    
    SUSPENDED --> ACTIVE : update_payment_method() + settle_balance()
    SUSPENDED --> CANCELLED : auto_cancel_timeout
    
    ACTIVE --> CANCELLED : cancel_subscription() [by Subscriber/Merchant]
    PAST_DUE --> CANCELLED : cancel_subscription()
    
    CANCELLED --> [*]
```

---

### 3.3 Escrow Status Lifecycle

```mermaid
stateDiagram-v2
    [*] --> INITIALIZED : create_escrow(buyer, seller, arbiter)
    INITIALIZED --> FUNDED : deposit_funds()
    
    FUNDED --> DELIVERED : submit_delivery_proof()
    
    DELIVERED --> RELEASED : approve_milestone()
    FUNDED --> RELEASED : release_without_dispute()
    
    FUNDED --> DISPUTED : raise_dispute()
    DELIVERED --> DISPUTED : raise_dispute()
    
    DISPUTED --> RESOLVED : arbiter_resolve(split_rule)
    DISPUTED --> REFUNDED : arbiter_refund_buyer()
    
    RELEASED --> [*]
    REFUNDED --> [*]
    RESOLVED --> [*]
```

---

### 3.4 Merchant Withdrawal Request Status

```mermaid
stateDiagram-v2
    [*] --> INITIATED : request_withdrawal(amount, destination)
    
    INITIATED --> INSTANT_APPROVED : amount <= instant_limit
    INITIATED --> PENDING_TIMELOCK : amount > instant_limit
    
    PENDING_TIMELOCK --> APPROVED : timelock_elapsed [no admin objection]
    PENDING_TIMELOCK --> REJECTED : admin_flag_suspicious()
    
    INSTANT_APPROVED --> EXECUTED : dispatch_token_transfer()
    APPROVED --> EXECUTED : execute_withdrawal()
    
    REJECTED --> CANCELLED : funds_returned_to_merchant_account
    
    EXECUTED --> [*]
    CANCELLED --> [*]
```

---

### 3.5 Ticket Token Lifecycle

```mermaid
stateDiagram-v2
    [*] --> MINTED : issue_event_tickets(event_id, supply, base_price)
    MINTED --> ACTIVE_HELD : purchase_primary()
    
    ACTIVE_HELD --> LISTED_RESALE : list_for_resale() [price <= cap]
    LISTED_RESALE --> ACTIVE_HELD : cancel_listing()
    LISTED_RESALE --> ACTIVE_HELD : purchase_resale() [ownership transferred]
    
    ACTIVE_HELD --> CHECKED_IN : verify_ticket_qr() [at event gate]
    
    ACTIVE_HELD --> REVOKED : event_cancelled_or_refunded
    LISTED_RESALE --> REVOKED : event_cancelled
    
    CHECKED_IN --> EXPIRED : event_concluded
    REVOKED --> [*]
    EXPIRED --> [*]
```
