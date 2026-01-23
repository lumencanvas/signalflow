# CLASP Documentation Cleanup and Update Plan
**Date:** January 23, 2026  
**Status:** 📋 **PLANNING**

---

## Executive Summary

This document outlines the plan to clean up and update all documentation in the CLASP monorepo to reflect:
1. Protocol version consolidation (version 1.0)
2. Recent code changes and fixes
3. Architecture clarifications
4. Terminology updates
5. Missing information
6. Outdated content

**Goal:** Ensure all documentation is accurate, consistent, and up-to-date with the current implementation.

---

## Part 1: Version Consolidation Updates

### 1.1 Protocol Specification Files

**Files to Update:**
- [x] `CLASP-Protocol.md` - ✅ Already consolidated
- [x] `CLASP-Protocol-v3.md` - ✅ Already removed (merged)
- [ ] `CLASP-QuickRef.md` - Check for version references

**Changes Needed:**
- [ ] Remove all "v3" references
- [ ] Update version to "1.0" consistently
- [ ] Clarify encoding version vs protocol version
- [ ] Update WebSocket subprotocol from `clasp.v3` to `clasp`
- [ ] Update example messages to use version 1

**Priority:** 🔴 **CRITICAL** - Version confusion must be resolved

---

### 1.2 Code Comments and Documentation

**Files to Review:**
- [ ] All Rust crate READMEs
- [ ] All Rust source files (doc comments)
- [ ] JavaScript/TypeScript files
- [ ] Python files
- [ ] C files (if any)

**Changes Needed:**
- [ ] Update `PROTOCOL_VERSION` references
- [ ] Update `ENCODING_VERSION` references
- [ ] Remove "v3" from comments
- [ ] Update subprotocol references
- [ ] Clarify version vs encoding distinction

**Priority:** 🟠 **HIGH** - Code docs should match spec

---

### 1.3 Website Documentation

**Files to Update:**
- [x] `site/src/components/SpecSection.vue` - ✅ Already updated
- [ ] Other Vue components (check for version refs)
- [ ] Site markdown files

**Changes Needed:**
- [ ] Remove "v3" from public-facing docs
- [ ] Update version numbers
- [ ] Update subprotocol references
- [ ] Update example code

**Priority:** 🟠 **HIGH** - Public docs must be accurate

---

## Part 2: Architecture Documentation Updates

### 2.1 Router Architecture

**Files to Update:**
- [ ] `docs/architecture.md`
- [ ] `README.md` (architecture section)
- [ ] `docs/guides/bridge-setup.md`
- [ ] `docs/guides/desktop-app-servers.md`

**Changes Needed:**
- [ ] Clarify router vs protocol connections
- [ ] Explain "internal" router concept
- [ ] Document P2P signaling role
- [ ] Clarify STUN/TURN (router is NOT STUN server)
- [ ] Update terminology (server → router, server → protocol connection)

**Priority:** 🔴 **CRITICAL** - Architecture confusion must be resolved

---

### 2.2 Protocol Connections vs Bridges

**Files to Update:**
- [ ] `docs/guides/desktop-app-servers.md`
- [ ] `docs/guides/bridge-setup.md`
- [ ] `docs/index.md`
- [ ] `README.md`

**Changes Needed:**
- [ ] Clarify: Protocol Connections (to CLASP router) vs Direct Bridges (bypass router)
- [ ] Explain bidirectional nature
- [ ] Document auto-bridge creation
- [ ] Update UI terminology references

**Priority:** 🔴 **CRITICAL** - Terminology confusion must be resolved

---

### 2.3 P2P Architecture

**Files to Update:**
- [ ] `CLASP-Protocol.md` (P2P section)
- [ ] `docs/guides/advanced/p2p-setup.md` (if exists)
- [ ] `.internal/P2P-FIX-SUMMARY-2026-01-23.md` (reference)

**Changes Needed:**
- [ ] Document ICE candidate exchange
- [ ] Clarify router's signaling role
- [ ] Explain STUN/TURN requirements
- [ ] Document connection state management
- [ ] Update with recent fixes

**Priority:** 🟠 **HIGH** - P2P is core feature

---

## Part 3: Implementation Updates

### 3.1 Recent Fixes Documentation

**Files to Create/Update:**
- [ ] `docs/guides/troubleshooting.md` (P2P section)
- [ ] `CHANGELOG.md` (if exists, or create)
- [ ] Update relevant guides with fixes

**Changes Needed:**
- [ ] Document P2P ICE candidate fix
- [ ] Document version consolidation
- [ ] Document any other recent fixes
- [ ] Add troubleshooting tips

**Priority:** 🟡 **MEDIUM** - Help users avoid known issues

---

### 3.2 Feature Completeness

**Files to Review:**
- [ ] `README.md` (feature list)
- [ ] `docs/index.md` (feature list)
- [ ] Protocol spec (promises vs reality)

**Changes Needed:**
- [ ] Verify all listed features are implemented
- [ ] Mark experimental features
- [ ] Update status of in-progress features
- [ ] Remove unimplemented features (or mark clearly)

**Priority:** 🟠 **HIGH** - Don't promise what's not delivered

---

## Part 4: Terminology Updates

### 4.1 Consistent Terminology

**Terms to Standardize:**

| Old Term | New Term | Files to Update |
|----------|----------|-----------------|
| "CLASP Server" | "CLASP Router" | All docs |
| "ADD SERVER" | "ADD PROTOCOL" | UI docs, guides |
| "OSC Server" | "OSC Connection" | When connected to CLASP |
| "Protocol Bridges" | "Protocol-to-Protocol Bridges" | When distinguishing |
| "OUTPUT TARGETS" | "Saved Destinations" | UI docs |

**Files to Update:**
- [ ] All markdown files in `docs/`
- [ ] `README.md`
- [ ] All crate READMEs
- [ ] Website content
- [ ] Example code comments

**Priority:** 🔴 **CRITICAL** - Terminology must be consistent

---

### 4.2 Signal Type Terminology

**Terms to Clarify:**
- [ ] "Signal" vs "Message" vs "Value"
- [ ] "Param" vs "Parameter"
- [ ] "Event" vs "Trigger"
- [ ] "Stream" vs "Continuous Data"
- [ ] "Gesture" vs "Phased Input"
- [ ] "Timeline" vs "Automation"

**Files to Update:**
- [ ] Protocol spec
- [ ] API documentation
- [ ] Guides
- [ ] Examples

**Priority:** 🟡 **MEDIUM** - Clarity is important

---

## Part 5: Missing Documentation

### 5.1 Missing Core Documentation

**Documents to Create:**
- [ ] `docs/api/overview.md` - API overview
- [ ] `docs/api/common/signal-types.md` - Signal type guide
- [ ] `docs/api/common/addressing.md` - Addressing guide
- [ ] `docs/api/common/state-management.md` - State management guide
- [ ] `docs/api/common/discovery.md` - Discovery guide
- [ ] `docs/api/common/security.md` - Security guide
- [ ] `docs/api/common/timing.md` - Timing guide
- [ ] `docs/api/common/p2p.md` - P2P guide

**Priority:** 🟠 **HIGH** - Core concepts need documentation

---

### 5.2 Missing Language-Specific Documentation

**Documents to Create:**
- [ ] `docs/api/rust/getting-started.md`
- [ ] `docs/api/rust/client-api.md`
- [ ] `docs/api/javascript/getting-started.md`
- [ ] `docs/api/javascript/browser-api.md`
- [ ] `docs/api/python/getting-started.md`
- [ ] `docs/api/python/async-api.md`

**Priority:** 🟠 **HIGH** - Language docs are essential

---

### 5.3 Missing Use Case Documentation

**Documents to Create:**
- [ ] `docs/guides/use-cases/live-performance.md`
- [ ] `docs/guides/use-cases/installation-art.md`
- [ ] `docs/guides/use-cases/home-automation.md`
- [ ] `docs/guides/use-cases/software-integration.md`
- [ ] `docs/guides/use-cases/embedded-systems.md`

**Priority:** 🟡 **MEDIUM** - Use cases help users understand

---

### 5.4 Missing Protocol Guides

**Documents to Create/Update:**
- [ ] `docs/guides/protocols/mqtt-integration.md` (if missing)
- [ ] `docs/guides/protocols/http-integration.md` (if missing)
- [ ] `docs/guides/protocols/websocket-bridge.md` (if missing)
- [ ] `docs/guides/protocols/socketio-bridge.md` (if missing)
- [ ] `docs/guides/protocols/sacn-integration.md` (if missing)

**Priority:** 🟡 **MEDIUM** - Protocol guides help integration

---

## Part 6: Outdated Content Removal

### 6.1 Deprecated Features

**Content to Remove/Update:**
- [ ] References to removed features
- [ ] Old API examples that no longer work
- [ ] Outdated configuration options
- [ ] Deprecated command-line flags

**Files to Review:**
- [ ] All documentation files
- [ ] Example code
- [ ] README files

**Priority:** 🟡 **MEDIUM** - Outdated content confuses users

---

### 6.2 Duplicate Content

**Content to Consolidate:**
- [ ] Multiple explanations of same concept
- [ ] Duplicate examples
- [ ] Redundant guides
- [ ] Overlapping documentation

**Files to Review:**
- [ ] `docs/` directory
- [ ] README files
- [ ] Crate READMEs

**Priority:** 🟢 **LOW** - Consolidation improves maintainability

---

## Part 7: Code Example Updates

### 7.1 Example Code Review

**Files to Review:**
- [ ] `examples/rust/`
- [ ] `examples/js/`
- [ ] `examples/python/`
- [ ] Documentation code blocks
- [ ] README examples

**Changes Needed:**
- [ ] Update to use version 1.0
- [ ] Update to use correct terminology
- [ ] Fix broken examples
- [ ] Add error handling
- [ ] Add comments
- [ ] Test all examples

**Priority:** 🟠 **HIGH** - Examples must work

---

### 7.2 Example Completeness

**Improvements Needed:**
- [ ] Add missing examples (P2P, bundles, etc.)
- [ ] Complete partial examples
- [ ] Add real-world scenarios
- [ ] Add error handling examples
- [ ] Add best practices examples

**Priority:** 🟡 **MEDIUM** - Complete examples are helpful

---

## Part 8: Cross-Reference Updates

### 8.1 Internal Links

**Files to Review:**
- [ ] All markdown files
- [ ] README files
- [ ] Website content

**Changes Needed:**
- [ ] Fix broken internal links
- [ ] Update link targets (if files moved)
- [ ] Add missing cross-references
- [ ] Verify all links work

**Priority:** 🟡 **MEDIUM** - Broken links frustrate users

---

### 8.2 External Links

**Files to Review:**
- [ ] All documentation files
- [ ] README files

**Changes Needed:**
- [ ] Verify external links work
- [ ] Update broken links
- [ ] Add missing links (crates.io, npm, PyPI)
- [ ] Update version numbers in links

**Priority:** 🟢 **LOW** - External links should work

---

## Part 9: Formatting and Style

### 9.1 Markdown Consistency

**Files to Review:**
- [ ] All markdown files

**Changes Needed:**
- [ ] Consistent heading levels
- [ ] Consistent code block formatting
- [ ] Consistent table formatting
- [ ] Consistent list formatting
- [ ] Consistent emphasis (bold/italic)

**Priority:** 🟢 **LOW** - Consistency improves readability

---

### 9.2 Code Block Formatting

**Files to Review:**
- [ ] All documentation files

**Changes Needed:**
- [ ] Language tags on all code blocks
- [ ] Consistent indentation
- [ ] Proper syntax highlighting
- [ ] Line numbers where helpful
- [ ] Comments in code examples

**Priority:** 🟢 **LOW** - Good formatting helps understanding

---

## Part 10: Testing Documentation

### 10.1 Test Documentation

**Files to Create/Update:**
- [ ] `docs/contributing/testing.md` (if missing)
- [ ] Test file documentation
- [ ] Test execution guide

**Changes Needed:**
- [ ] Document how to run tests
- [ ] Document test structure
- [ ] Document test coverage
- [ ] Document test requirements

**Priority:** 🟡 **MEDIUM** - Test docs help contributors

---

## Part 11: Implementation Checklist

### Phase 1: Critical Updates (Week 1)

**Version Consolidation:**
- [ ] Update `CLASP-QuickRef.md`
- [ ] Review and update all code comments
- [ ] Update website components
- [ ] Update example code

**Architecture Updates:**
- [ ] Update `docs/architecture.md`
- [ ] Update `README.md` architecture section
- [ ] Update `docs/guides/bridge-setup.md`
- [ ] Update `docs/guides/desktop-app-servers.md`

**Terminology:**
- [ ] Update all "server" → "router" references
- [ ] Update all "ADD SERVER" → "ADD PROTOCOL" references
- [ ] Update protocol connection terminology

**Priority:** 🔴 **CRITICAL** - Must be done first

---

### Phase 2: High Priority Updates (Week 2)

**P2P Documentation:**
- [ ] Document ICE candidate exchange
- [ ] Update P2P guides
- [ ] Add troubleshooting tips

**Feature Completeness:**
- [ ] Review and update feature lists
- [ ] Mark experimental features
- [ ] Update status of in-progress features

**Example Code:**
- [ ] Review all examples
- [ ] Fix broken examples
- [ ] Update to version 1.0
- [ ] Add error handling

**Priority:** 🟠 **HIGH** - Important for users

---

### Phase 3: Medium Priority Updates (Week 3-4)

**Missing Documentation:**
- [ ] Create core API documentation
- [ ] Create language-specific docs
- [ ] Create use case guides
- [ ] Create protocol guides

**Cross-References:**
- [ ] Fix broken links
- [ ] Add missing links
- [ ] Update link targets

**Formatting:**
- [ ] Standardize markdown formatting
- [ ] Improve code block formatting
- [ ] Improve table formatting

**Priority:** 🟡 **MEDIUM** - Improves documentation quality

---

### Phase 4: Low Priority Updates (Ongoing)

**Cleanup:**
- [ ] Remove deprecated content
- [ ] Consolidate duplicate content
- [ ] Improve formatting consistency
- [ ] Add test documentation

**Priority:** 🟢 **LOW** - Nice to have improvements

---

## Part 12: File-by-File Update List

### Root Documentation

- [ ] `README.md` - Update terminology, version, architecture
- [ ] `CLASP-Protocol.md` - ✅ Already updated
- [ ] `CLASP-QuickRef.md` - Review and update version references
- [ ] `CONTRIBUTING.md` - Review and update if needed

### Docs Directory

- [ ] `docs/index.md` - Update terminology, feature list
- [ ] `docs/architecture.md` - Update architecture explanation
- [ ] `docs/getting-started/README.md` - Review and update
- [ ] `docs/guides/bridge-setup.md` - Update terminology
- [ ] `docs/guides/desktop-app-servers.md` - Update terminology
- [ ] `docs/guides/protocol-mapping.md` - Review and update
- [ ] `docs/guides/troubleshooting.md` - Add P2P troubleshooting
- [ ] `docs/integrations/*.md` - Review and update
- [ ] `docs/protocols/*.md` - Review and update

### Crate READMEs

- [ ] `crates/clasp-core/README.md` - Update version references
- [ ] `crates/clasp-client/README.md` - Review and update
- [ ] `crates/clasp-router/README.md` - Review and update
- [ ] `crates/clasp-transport/README.md` - Review and update
- [ ] `crates/clasp-bridge/README.md` - Review and update
- [ ] `crates/clasp-wasm/README.md` - Review and update
- [ ] `crates/clasp-discovery/README.md` - Review and update
- [ ] `crates/clasp-cli/README.md` - Review and update

### Binding READMEs

- [ ] `bindings/js/packages/clasp-core/README.md` - Review and update
- [ ] `bindings/python/README.md` - Review and update

### Example Files

- [ ] `examples/README.md` - Review and update
- [ ] `examples/rust/*.rs` - Review and update examples
- [ ] `examples/js/*.js` - Review and update examples
- [ ] `examples/python/*.py` - Review and update examples

### Website

- [ ] `site/src/components/*.vue` - Review for version references
- [ ] `site/src/pages/*.vue` - Review and update
- [ ] `site/src/router.js` - Review if needed

---

## Part 13: Quality Assurance

### 13.1 Documentation Review Process

**Checklist:**
- [ ] All version references updated
- [ ] All terminology consistent
- [ ] All examples tested and working
- [ ] All links verified
- [ ] All code blocks formatted correctly
- [ ] All tables formatted correctly
- [ ] No broken references
- [ ] No duplicate content
- [ ] No outdated information

---

### 13.2 Testing Documentation

**Process:**
- [ ] Run all code examples
- [ ] Verify all commands work
- [ ] Test all links
- [ ] Review for clarity
- [ ] Review for completeness
- [ ] Review for accuracy

---

## Part 14: Maintenance Plan

### 14.1 Ongoing Maintenance

**Tasks:**
- [ ] Review docs with each code change
- [ ] Update examples when APIs change
- [ ] Fix broken links regularly
- [ ] Update version numbers
- [ ] Incorporate user feedback
- [ ] Keep terminology consistent

---

## Part 15: Success Criteria

### Documentation Quality

- [ ] All version references are "1.0" or "1"
- [ ] All terminology is consistent
- [ ] All examples are tested and working
- [ ] All links are verified
- [ ] No broken references
- [ ] No outdated information
- [ ] Architecture is clearly explained

### Documentation Completeness

- [ ] Core concepts documented
- [ ] All APIs documented
- [ ] All use cases covered
- [ ] All protocols covered
- [ ] Examples for major features
- [ ] Troubleshooting guides

### Documentation Usability

- [ ] Easy to navigate
- [ ] Clear structure
- [ ] Good search functionality
- [ ] Mobile-friendly
- [ ] Accessible

---

**Last Updated:** January 23, 2026  
**Status:** 📋 Planning complete, ready for implementation
