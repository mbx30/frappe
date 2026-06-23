# Tauri v2 Rendering & Performance Optimization Project

**Status**: Planning  
**Total Issues**: 16  
**Estimated Duration**: 10 weeks  
**Owner**: @mbx30

---

## 📋 Project Overview

Implementation of "Engineering Smooth Desktop Applications with Tauri v2: Rendering Pipeline Optimization, Rust Performance, and Security Hardening" based on technical paper analysis.

**Paper Goals**:
- Sustained 60 fps rendering
- Sub-100 ms IPC round-trips  
- Absence of UI jank
- Security-hardened capability model
- Minimal binary size & memory footprint

---

## 🎯 Critical Priority Sequence

### 🔴 Phase 1: Foundation & Security (Weeks 1-2)

#### Week 1.1: Measurement Infrastructure
- **#298** - Performance Testing & Metrics Collection
  - Status: Not Started
  - Est. Time: 1 week
  - Why: Can't optimize blind. Must establish baselines first.
  - Blocks: All other optimization work
  - Deliverables: Flamegraph setup, Lighthouse CI, performance dashboard

#### Week 1.2: Build Configuration
- **#288** - Cargo.toml Release Profile Configuration
  - Status: Not Started
  - Est. Time: 1 day
  - Why: Simple change, 10-30% speedup across all backend code
  - Depends on: Nothing
  - Priority: CRITICAL (affects all subsequent work)
  - Deliverables: Optimized Cargo.toml, link time measurements

### 🟠 Phase 1.2: Security Foundation (Weeks 1.5-2)

- **#295** - Content Security Policy (CSP) Hardening
  - Status: Not Started
  - Est. Time: 3-4 days
  - Why: Security gate. Prevents shipping with XSS vulnerabilities
  - Depends on: Nothing
  - Priority: CRITICAL (must not regress)
  - Deliverables: Strict CSP configuration, no unsafe-inline/eval

- **#294** - Capability-Based Access Control  
  - Status: Not Started
  - Est. Time: 3-4 days
  - Why: Security foundation + minimal surface = faster IPC
  - Depends on: Nothing
  - Priority: CRITICAL (must not regress)
  - Deliverables: Minimal capability set, path scopes

---

### 🟠 Phase 2: IPC Optimization (Weeks 3-5) — BIGGEST BOTTLENECK

#### Week 3: Async Foundations

- **#289** - Async Command Architecture (spawn_blocking)
  - Status: Not Started
  - Est. Time: 2-3 days
  - Why: Prevents backend from blocking other commands. Foundation for IPC latency.
  - Depends on: #288
  - Priority: CRITICAL (blocks #291, #292, #293)
  - Deliverables: All commands use spawn_blocking, no blocking operations on async runtime

#### Week 3-4: Batching

- **#291** - Command Batching
  - Status: Not Started
  - Est. Time: 1 week
  - Why: Single biggest IPC performance win (50-100x reduction for N=100)
  - Depends on: #289
  - Priority: HIGH (major latency reduction)
  - Deliverables: Batch variants of hot path commands, ~50% IPC latency reduction

#### Week 4-5: Binary & Streaming

- **#293** - Binary Payload Handling (File/Protocol Serving)
  - Status: Not Started
  - Est. Time: 3-4 days
  - Why: Eliminates base64 overhead for image-heavy apps
  - Depends on: #289
  - Priority: MEDIUM-HIGH (depends on use case)
  - Deliverables: File-based asset serving, custom protocol handler

- **#292** - Channel API for Streaming
  - Status: Not Started
  - Est. Time: 1 week
  - Why: Replace polling/emit with efficient streaming
  - Depends on: #289
  - Priority: MEDIUM (depends on use case)
  - Deliverables: Streaming search, progress updates, real-time data

---

### 🟡 Phase 3: Frontend Optimization (Weeks 5-8)

#### Week 5-6: Framework & Assets

- **#284** - Framework Selection & Memoization
  - Status: Not Started
  - Est. Time: 2-3 days (analysis) + ? (if migration)
  - Why: Framework choice affects all downstream frontend work
  - Depends on: #298 (for metrics)
  - Priority: HIGH (affects strategy for #285-287)
  - Deliverables: Framework analysis, memoization patterns if React

- **#285** - Asset Pipeline Optimization
  - Status: Not Started
  - Est. Time: 1 week
  - Why: WebP/AVIF = 30-50% size reduction, font subsetting = 200KB → 30KB
  - Depends on: #284
  - Priority: HIGH (significant startup improvement)
  - Deliverables: WebP/AVIF images, subsetted fonts, code splitting

#### Week 6-8: Rendering Performance

- **#290** - State Management (Mutex/RwLock Optimization)
  - Status: Not Started
  - Est. Time: 3-4 days
  - Why: Reduces lock contention on high-throughput backends
  - Depends on: #289
  - Priority: MEDIUM (only if high concurrency)
  - Deliverables: Optimized locking strategy, dashmap evaluation

- **#286** - Virtual Scrolling
  - Status: Not Started
  - Est. Time: 3-4 days
  - Why: Eliminates jank for lists >500 items, 60 fps improvement
  - Depends on: #284
  - Priority: MEDIUM (depends on list size)
  - Deliverables: @tanstack/virtual implementation, 60 fps list scrolling

- **#287** - Animation Optimization (GPU Compositing)
  - Status: Not Started
  - Est. Time: 2-3 days
  - Why: GPU compositor = smooth animations independent of JS
  - Depends on: #284
  - Priority: MEDIUM (depends on animation volume)
  - Deliverables: transform/opacity animations, GPU layer promotion

---

### 🟢 Phase 4: Hardening & Final Audit (Weeks 8-10)

- **#296** - IPC Surface Minimization & Path Validation
  - Status: Not Started
  - Est. Time: 1 week
  - Why: Defense-in-depth security hardening
  - Depends on: #294, #295
  - Priority: HIGH (before shipping)
  - Deliverables: Input validation, path canonicalization, security tests

- **#297** - Pre-Release Security Audit Checklist
  - Status: Not Started
  - Est. Time: 2-3 days
  - Why: Final gate before shipping
  - Depends on: All others
  - Priority: CRITICAL (gate for release)
  - Deliverables: Audit completion, sign-off

- **#299** - Meta: Implementation Plan (Tracking)
  - Status: Created
  - Est. Time: Ongoing
  - Purpose: Links all issues, tracks overall progress
  - Deliverables: Updated status tracking

---

## 🚀 Minimal Viable Path (4 weeks)

If short on time, do these first for 50-70% IPC latency reduction:

1. **#298** (metrics) — 1 week
2. **#288** (Cargo) — 1 day
3. **#295** (CSP) — 4 days
4. **#294** (capabilities) — 4 days
5. **#289** (async) — 3 days
6. **#291** (batching) — 1 week

**Result**: Solid foundation, major IPC gains, secure

---

## 📊 Phase Timeline

```
Week 1-2:    [====] Foundation & Security
Week 3-5:    [=====] IPC Optimization (BIGGEST WINS HERE)
Week 5-8:    [====] Frontend Optimization
Week 8-10:   [====] Hardening & Audit
```

---

## 🎯 Success Criteria

### Performance Goals
- [ ] Startup time: <500ms
- [ ] Binary size: <50MB (bundled)
- [ ] Memory footprint: <100MB idle
- [ ] IPC latency: <50ms average, <100ms p99
- [ ] UI interactions: 60 fps sustained

### Security Goals
- [ ] CSP: strict (no unsafe-inline/eval)
- [ ] Capabilities: minimal (only used)
- [ ] IPC: validated inputs, no path traversal
- [ ] Dependencies: no known vulns

### Testing Goals
- [ ] Performance regressions detected in CI
- [ ] Security audit passing
- [ ] Offline functionality verified
- [ ] Cross-platform testing (Windows/macOS/Linux)

---

## 🔗 Issue Dependencies

```
#288 (Cargo) ────────┐
                     ├─→ #289 (Async) ──┬─→ #291 (Batching)
                     │                  ├─→ #292 (Channel)
#295 (CSP) ─────┐    ├─→ #290 (State)   └─→ #293 (Binary)
                │    │
#294 (Cap) ─────┼──→ #296 (Surface) ──→ #297 (Audit)
                │
#298 (Metrics)──┴─→ #284 (Framework) ──┬─→ #285 (Assets)
                                       ├─→ #286 (Virtual)
                                       └─→ #287 (Animation)

#299 (Meta) = tracking issue for all
```

---

## 📝 How to Use This Document

1. **Create GitHub Project** using web UI: https://github.com/mbx30/frappe/projects
2. **Copy this structure** into project description
3. **Link issues** to project in priority order
4. **Create columns**:
   - 📋 Not Started (default)
   - 🔄 In Progress
   - ✅ Done
   - 🚫 Blocked

5. **Track progress**: Move issues across columns as work progresses
6. **Update status** in this document weekly

---

## 📚 Paper References

- **Paper Title**: Engineering Smooth Desktop Applications with Tauri v2: Rendering Pipeline Optimization, Rust Performance, and Security Hardening
- **Key Sections**:
  - Section 3: Rendering pipeline optimization
  - Section 4: Rust backend performance patterns
  - Section 5: IPC performance patterns  
  - Section 6: Security hardening
  - Section 7: Integrated best practices

---

## 👥 Team Assignments

| Issue | Owner | Reviewer |
|-------|-------|----------|
| #298  | ?     | ?        |
| #288  | ?     | ?        |
| #295  | ?     | ?        |
| #294  | ?     | ?        |
| #289  | ?     | ?        |
| #291  | ?     | ?        |
| #293  | ?     | ?        |
| #292  | ?     | ?        |
| #284  | ?     | ?        |
| #285  | ?     | ?        |
| #290  | ?     | ?        |
| #286  | ?     | ?        |
| #287  | ?     | ?        |
| #296  | ?     | ?        |
| #297  | ?     | ?        |
| #299  | @mbx30 | @mbx30 |

---

## 📞 Questions?

Refer to the specific GitHub issue for detailed acceptance criteria and implementation guidance. Each issue contains:
- Full description
- Acceptance criteria (checkboxes)
- Implementation details
- Related issues
- Paper section references
