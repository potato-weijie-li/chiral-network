# Feature Proposals

This directory contains feature proposals for Chiral Network following our **doc-first development model**.

## Purpose

Before implementing new features (excluding minor fixes, bug fixes, or GUI changes), we create a proposal document that:

1. **Explains the feature** - What is being proposed?
2. **Justifies the need** - Why is this feature necessary?
3. **Ensures alignment** - Does it align with core principles?
4. **Plans implementation** - How will it be built?
5. **Considers alternatives** - What other approaches were considered?

## When to Create a Proposal

### ✅ Create a Proposal For:
- New major features or capabilities
- Changes to core architecture
- Protocol additions or modifications
- New integrations or dependencies
- Features that could impact security or privacy
- Anything that affects the project's mission or scope

### ❌ No Proposal Needed For:
- Bug fixes
- Minor UI improvements
- Documentation updates
- Performance optimizations
- Refactoring existing code
- Test additions

## Proposal Process

### 1. Create Proposal Document

Use the template below or see existing proposals for examples:

```markdown
# Feature Proposal: [Feature Name]

**Status:** DRAFT | APPROVED | REJECTED | IMPLEMENTED  
**Author:** [Your Name]  
**Date:** [Date]  
**Related Issue:** [Link or reference]

## Executive Summary
Brief overview of the feature.

## Problem Statement
What problem does this solve?

## Proposed Solution
Detailed description of the feature.

## Why This Feature?
Justification and alignment with project goals.

## Implementation Plan
High-level implementation steps.

## Alternatives Considered
What other approaches were evaluated?

## Security & Privacy Considerations
Any concerns or mitigations.

## Testing Strategy
How will this be tested?

## Documentation Requirements
What docs need to be updated?

## Success Metrics
How do we measure success?
```

### 2. Submit for Review

1. Create a PR with your proposal in `docs/proposals/`
2. Tag relevant maintainers for review
3. Discuss in PR comments
4. Address feedback

### 3. Get Approval

Proposal is marked as **APPROVED** when:
- Maintainers agree on the approach
- Security/privacy concerns addressed
- Implementation plan is clear
- Aligns with project principles

### 4. Implementation

Once approved:
1. Update proposal status to **APPROVED**
2. Create implementation issues if needed
3. Begin development
4. Reference proposal in PRs
5. Update proposal when implemented

## Core Principles (Reference)

All proposals must align with Chiral Network's core principles:

1. **Fully Decentralized P2P** - No centralized servers
2. **BitTorrent-Style Sharing** - Instant seeding, continuous availability
3. **Non-Commercial** - No marketplace, pricing, or trading
4. **Privacy-First** - Strong privacy and anonymity features
5. **Legitimate Use Only** - Designed for legal file sharing
6. **Blockchain Integration** - Ethereum-compatible blockchain

### What We DON'T Build

- ❌ General-purpose download managers
- ❌ VPN or general anonymity networks
- ❌ Marketplace or trading platforms
- ❌ Global file search/discovery (piracy risk)
- ❌ Commercial features (pricing, payments)
- ❌ Social features (comments, likes, reviews)

## Active Proposals

| Proposal | Status | Author | Date |
|----------|--------|--------|------|
| [Multi-Protocol Support](multi-protocol-support.md) | DRAFT | Copilot Agent | 2025-10-21 |

## Approved & Implemented

_(None yet)_

## Rejected Proposals

_(None yet)_

## Questions?

- Read [CLAUDE.md](../../CLAUDE.md) for development guidelines
- Check [Contributing Guide](../contributing.md) for contribution process
- Join discussions on [Zulip](https://brooknet.zulipchat.com/join/f3jj4k2okvlfpu5vykz5kkk5/)

---

**Remember:** The doc-first model helps us build the right features the right way, avoiding mission creep and maintaining project focus.
