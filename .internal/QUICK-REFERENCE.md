# Quick Reference: Protocol-Centric Model

**For:** Developers implementing the new model  
**See Also:** `.internal/MASTER-CONSOLIDATION-PLAN.md` for detailed updates

---

## The Model (One Sentence)

**Organize by Protocol, Configure by Role, Distinguish Connections from Direct Bridges.**

---

## Terminology Quick Reference

| Old Term | New Term | Context |
|----------|----------|---------|
| "ADD SERVER" | "ADD PROTOCOL" | Button/modal |
| "CLASP Server" | "CLASP Router" | Router component |
| "OSC Server" | "OSC Connection" | When connected to CLASP |
| "Protocol Bridges" | "Protocol-to-Protocol Bridges" | Direct connections |
| "OUTPUT TARGETS" | "Saved Destinations" | Saved configs |

---

## UI Structure

```
Sidebar:
  ├── CLASP ROUTERS
  │   └── [+ ADD ROUTER]
  │
  ├── PROTOCOL CONNECTIONS
  │   └── [+ ADD PROTOCOL]
  │
  └── DIRECT CONNECTIONS
      └── [+ CREATE DIRECT BRIDGE]
```

---

## Key Files to Update

### Critical (Must Fix First)
1. `apps/bridge/electron/main.js` - Fix "internal" router connection
2. `apps/bridge/src/app.js` - Separate routers, update handlers
3. `apps/bridge/src/index.html` - Reorganize UI, update modals

### Documentation ✅ COMPLETED (2026-01-22)
1. `README.md` - ✅ Updated terminology and examples
2. `docs/guides/bridge-setup.md` - ✅ Updated desktop app instructions
3. `docs/guides/desktop-app-servers.md` - ✅ Rewritten for protocol-centric model
4. `crates/clasp-cli/README.md` - ✅ Updated terminology
5. `docs/index.md` - ✅ Updated terminology
6. `docs/protocols/README.md` - ✅ Updated terminology

---

## Implementation Phases

1. **Phase 1:** Fix "internal" router connection ✅ IMPLEMENTED (code in main.js)
2. **Phase 2:** Reorganize UI (HIGH) - Pending UI changes
3. **Phase 3:** Update documentation ✅ COMPLETED (2026-01-22)
4. **Phase 4:** Additional features (MEDIUM)

---

## Key Concepts

- **Protocol Connections:** Connect protocols to CLASP router (bidirectional)
- **Direct Bridges:** Protocol-to-protocol (bypass CLASP)
- **Role is Configuration:** Server/client/device is a setting, not organization
- **Transports are Router Settings:** WebSocket/QUIC/TCP are router config

---

*See `.internal/MASTER-CONSOLIDATION-PLAN.md` for complete details.*
